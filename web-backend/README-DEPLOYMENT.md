# 🚀 Rustify Web Deployment Guide - Render.com

This guide covers deploying the Rustify web application to Render.com hosting platform.

## 📋 Prerequisites

- Git repository with your code pushed to GitHub/GitLab
- Render.com account (free tier available)
- All dependencies properly configured

## 🔧 Deployment Steps

### 1. Repository Setup
Ensure your repository is pushed to GitHub/GitLab with the following structure:
```
ezp3/
├── web-backend/
│   ├── src/
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── render.yaml
├── dist/
│   ├── index.html
│   └── liquid-space-nebula.mp4
└── README.md
```

### 2. Render.com Configuration

1. **Sign up/Login** to [Render.com](https://render.com)
2. **Connect Repository**: Link your GitHub/GitLab account
3. **Create New Web Service**: Select "New" → "Web Service"
4. **Select Repository**: Choose your Rustify repository
5. **Configure Service**:
   - **Name**: `rustify-web-backend`
   - **Runtime**: Rust
   - **Build Command**: `cd web-backend && cargo build --release`
   - **Start Command**: `cd web-backend && ./target/release/rustify-web-backend`
   - **Plan**: Free (or paid for production)

### 3. Environment Variables

Set these environment variables in Render.com dashboard:
```bash
RUST_LOG=info
PORT=10000
RUSTIFY_BIND_HOST=0.0.0.0
```

### 4. Auto-Deploy Configuration

The `render.yaml` file is configured for automatic deployment:
- **Auto-deploy**: Enabled from main branch
- **Health Check**: Root path `/`
- **Disk**: 1GB for free tier

## 🌐 Access Your Application

After successful deployment:
- **URL**: `https://your-service-name.onrender.com`
- **Features**: Full web interface with video conversion
- **Backend**: Rust-powered API with WebSocket support

## 🔍 Monitoring & Logs

- **Logs**: Available in Render.com dashboard
- **Health**: Automatic health checks on root path
- **Metrics**: Built-in monitoring for free tier

## 🛠 Troubleshooting

### Common Issues:

1. **Build Failures**:
   - Check Cargo.toml dependencies
   - Verify Rust version compatibility
   - Review build logs in Render dashboard

2. **Static Files Not Loading**:
   - Ensure `dist/` folder is in repository root
   - Check static file paths in main.rs
   - Verify Dockerfile copies files correctly

3. **WebSocket Connection Issues**:
   - Ensure CORS is properly configured
   - Check environment variables
   - Verify port binding (0.0.0.0 for production)

### Debug Commands:
```bash
# Local testing with production config
export RUSTIFY_BIND_HOST=0.0.0.0
export PORT=10000
cargo run

# Check static file serving
curl http://localhost:10000/
curl http://localhost:10000/api/health
```

## 📁 File Structure for Deployment

**Key files for Render.com:**
- `web-backend/render.yaml` - Render configuration
- `web-backend/Dockerfile` - Container configuration  
- `web-backend/src/main.rs` - Server with production settings
- `dist/index.html` - Frontend application
- `dist/liquid-space-nebula.mp4` - Video background

## 🎯 Production Considerations

1. **Security**: OWASP-compliant headers enabled
2. **Performance**: Compression and caching configured
3. **Monitoring**: Structured logging with tracing
4. **Scalability**: Stateless design for horizontal scaling

## 📞 Support

For deployment issues:
1. Check Render.com documentation
2. Review application logs
3. Verify environment configuration
4. Test locally with production settings

---
**Note**: This configuration is optimized for Render.com's free tier. For production workloads, consider upgrading to paid plans for better performance and reliability.