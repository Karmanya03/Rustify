# Deployment Instructions for Rustify

## Render.com Deployment

### Prerequisites
- GitHub repository with the Rustify code
- Render.com account

### Deployment Steps

1. **Connect Repository**: 
   - Go to Render.com dashboard
   - Create new "Web Service"
   - Connect your GitHub repository

2. **Environment Configuration**:
   ```
   Build Command: docker build -f web-backend/Dockerfile -t rustify .
   Start Command: ./web-backend
   ```

3. **Environment Variables** (Optional):
   ```
   RUST_LOG=info
   PORT=10000
   ```

### Docker Requirements
The Dockerfile now includes:
- ✅ Python 3 and pip3
- ✅ yt-dlp installation via pip3
- ✅ FFmpeg for media processing
- ✅ SSL certificates and dependencies

### Troubleshooting

**Error: "No such file or directory (os error 2)"**
- Ensure Dockerfile includes Python and yt-dlp installation
- Verify the deployment is using the updated Dockerfile

**Error: "yt-dlp not found"**
- Check build logs for Python/pip installation errors
- Ensure Docker build completes successfully

### Local Development
1. Install yt-dlp: `pip install yt-dlp`
2. Run: `cargo run` in web-backend directory
3. Access: http://localhost:8080

### Production Verification
After deployment, the application will:
1. Check for yt-dlp availability on startup
2. Display warning if not found
3. Show proper error messages for failed conversions
