# Rustify YouTube Downloader Setup Script for Windows
# This script installs the required dependencies for YouTube downloading

Write-Host "🚀 Setting up Rustify YouTube Downloader dependencies..." -ForegroundColor Green

# Check if Python is installed
$pythonCmd = $null
if (Get-Command python -ErrorAction SilentlyContinue) {
    $pythonCmd = "python"
} elseif (Get-Command python3 -ErrorAction SilentlyContinue) {
    $pythonCmd = "python3"
}

if (-not $pythonCmd) {
    Write-Host "❌ Python is not installed. Please install Python first:" -ForegroundColor Red
    Write-Host "   Download from https://python.org" -ForegroundColor Yellow
    Write-Host "   Make sure to check 'Add Python to PATH' during installation" -ForegroundColor Yellow
    exit 1
}

# Check if pip is available
$pipCmd = $null
if (Get-Command pip -ErrorAction SilentlyContinue) {
    $pipCmd = "pip"
} elseif (Get-Command pip3 -ErrorAction SilentlyContinue) {
    $pipCmd = "pip3"
}

if (-not $pipCmd) {
    Write-Host "❌ pip is not installed. Please install pip first." -ForegroundColor Red
    exit 1
}

# Install yt-dlp
Write-Host "📦 Installing yt-dlp..." -ForegroundColor Blue
try {
    & $pipCmd install --upgrade yt-dlp
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ yt-dlp installed successfully!" -ForegroundColor Green
    } else {
        throw "Installation failed"
    }
} catch {
    Write-Host "❌ Failed to install yt-dlp. Please try:" -ForegroundColor Red
    Write-Host "   pip install --upgrade yt-dlp" -ForegroundColor Yellow
    Write-Host "   or run PowerShell as Administrator" -ForegroundColor Yellow
    exit 1
}

# Check if ffmpeg is installed
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "⚠️  ffmpeg is not installed. Installing ffmpeg is recommended for audio conversion." -ForegroundColor Yellow
    Write-Host "   Download from https://ffmpeg.org" -ForegroundColor Yellow
    Write-Host "   or use chocolatey: choco install ffmpeg" -ForegroundColor Yellow
    Write-Host "   or use winget: winget install ffmpeg" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "   Without ffmpeg, you may have limited audio format options." -ForegroundColor Yellow
} else {
    Write-Host "✅ ffmpeg is already installed!" -ForegroundColor Green
}

# Test yt-dlp installation
Write-Host "🧪 Testing yt-dlp installation..." -ForegroundColor Blue
try {
    $version = & yt-dlp --version
    Write-Host "✅ yt-dlp is working! Version: $version" -ForegroundColor Green
} catch {
    Write-Host "❌ yt-dlp test failed. Please check your installation." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "🎉 Setup complete! You can now use Rustify to download YouTube videos and playlists." -ForegroundColor Green
Write-Host ""
Write-Host "📝 Usage:" -ForegroundColor Cyan
Write-Host "   1. Start the Rustify web server: cargo run" -ForegroundColor White
Write-Host "   2. Open your browser to http://localhost:3001" -ForegroundColor White
Write-Host "   3. Paste a YouTube URL and start downloading!" -ForegroundColor White
Write-Host ""
Write-Host "📋 Supported formats:" -ForegroundColor Cyan
Write-Host "   - MP3 (audio only) - various bitrates" -ForegroundColor White
Write-Host "   - WAV (audio only) - lossless quality" -ForegroundColor White
Write-Host "   - MP4 (video) - 360p to 1080p" -ForegroundColor White
Write-Host "   - WebM (video) - 360p to 1080p" -ForegroundColor White
Write-Host ""
Write-Host "🔗 Supports both individual videos and entire playlists!" -ForegroundColor Cyan
