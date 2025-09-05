# 🚀 Rustify - Render.com Deployment Guide

Deploy your Rustify YouTube converter to Render.com for **completely FREE** hosting!

## ✅ Prerequisites
- GitHub account (free)
- Render.com account (free, no billing required)

## 🚀 Quick Deployment Steps

### 1. Push to GitHub
```bash
# If not already in a repo, initialize it
git init
git add .
git commit -m "Initial Rustify commit"
git branch -M main
git remote add origin https://github.com/YOUR_USERNAME/rustify.git
git push -u origin main
```

### 2. Deploy on Render.com
1. Visit [render.com](https://render.com)
2. Sign up with your GitHub account
3. Click **"New +"** → **"Web Service"**
4. Connect your GitHub repository
5. Configure:
   - **Build Command**: `cargo build --release --bin rustify-web-backend`
   - **Start Command**: `./target/release/rustify-web-backend`
   - **Root Directory**: `web-backend`

### 3. Advanced Settings (Optional)
- **Environment Variables**: 
  - `RUST_LOG=info`
- **Auto-Deploy**: ✅ Enabled
- **Health Check Path**: `/api/health`

## 🌐 Your Live App
After deployment, your Rustify app will be available at:
**https://YOUR_APP_NAME.onrender.com**

## 💰 Cost
**Completely FREE!** 
- 750 hours/month free tier
- Auto-sleeps when idle to save hours
- No credit card required

## 🔧 Project Structure
```
web-backend/
├── Dockerfile          # Container configuration
├── render.yaml         # Render.com settings
├── .dockerignore       # Docker build exclusions
├── Cargo.toml          # Rust dependencies
└── src/                # Source code
    └── main.rs         # Web server (supports PORT env var)
```

## 🐛 Troubleshooting

### Build Issues
- Check build logs in Render dashboard
- Ensure all dependencies are in Cargo.toml

### App Not Starting
- Verify `/api/health` endpoint responds
- Check if PORT environment variable is handled

### Slow First Load
- Free tier apps sleep when idle
- First request may take 30-60 seconds

## 🎉 Features Included
- ✅ YouTube video conversion
- ✅ Multiple format support
- ✅ Real-time progress updates
- ✅ Secure file handling
- ✅ Animated UI with rain background
- ✅ 3D button styling
- ✅ OWASP security compliance

---
**Happy deploying! 🚀**
