# EzP3 Build Script for Windows PowerShell

param(
    [switch]$Release,
    [switch]$SkipTests,
    [switch]$SkipDesktop
)

Write-Host "🚀 Building EzP3 YouTube Converter..." -ForegroundColor Cyan

# Check if Rust is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust is not installed. Please install Rust first:" -ForegroundColor Red
    Write-Host "   Visit: https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

# Check if FFmpeg is installed
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "⚠️  FFmpeg is not found in PATH" -ForegroundColor Yellow
    Write-Host "   Please install FFmpeg:" -ForegroundColor Yellow
    Write-Host "   - Download from: https://ffmpeg.org/download.html" -ForegroundColor Yellow
    Write-Host "   - Or use chocolatey: choco install ffmpeg" -ForegroundColor Yellow
    Write-Host "   Continuing build anyway..." -ForegroundColor Yellow
}

# Create build directory
New-Item -ItemType Directory -Force -Path "dist" | Out-Null

$BuildMode = if ($Release) { "--release" } else { "" }
$TargetDir = if ($Release) { "release" } else { "debug" }

Write-Host "📦 Building core library..." -ForegroundColor Green
Set-Location core
cargo build $BuildMode
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Set-Location ..

Write-Host "🖥️  Building CLI application..." -ForegroundColor Green
Set-Location cli
cargo build $BuildMode
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Set-Location ..

# Copy CLI binary
$CliSource = "target\$TargetDir\ezp3.exe"
if (Test-Path $CliSource) {
    Copy-Item $CliSource "dist\" -Force
    Write-Host "✅ CLI binary copied to dist/" -ForegroundColor Green
}

Write-Host "🌐 Building web backend..." -ForegroundColor Green
Set-Location web-backend
cargo build $BuildMode
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Set-Location ..

# Copy web backend binary
$WebSource = "target\$TargetDir\ezp3-web-backend.exe"
if (Test-Path $WebSource) {
    Copy-Item $WebSource "dist\" -Force
    Write-Host "✅ Web backend binary copied to dist/" -ForegroundColor Green
}

# Build desktop app if Tauri is available and not skipped
if (-not $SkipDesktop) {
    if (Get-Command cargo-tauri -ErrorAction SilentlyContinue) {
        Write-Host "🖱️  Building desktop application..." -ForegroundColor Green
        Set-Location desktop
        if ($Release) {
            cargo tauri build
        } else {
            cargo tauri build --debug
        }
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Desktop app built successfully" -ForegroundColor Green
        }
        Set-Location ..
    } else {
        Write-Host "⚠️  Tauri CLI not found. Skipping desktop build." -ForegroundColor Yellow
        Write-Host "   Install with: cargo install tauri-cli" -ForegroundColor Yellow
    }
}

# Run tests
if (-not $SkipTests) {
    Write-Host "🧪 Running tests..." -ForegroundColor Green
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) {
        Write-Host "⚠️  Some tests failed, but build completed" -ForegroundColor Yellow
    }
}

Write-Host "✅ Build completed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "📁 Built artifacts:" -ForegroundColor Cyan
Write-Host "   CLI: .\dist\ezp3.exe" -ForegroundColor White
Write-Host "   Web Backend: .\dist\ezp3-web-backend.exe" -ForegroundColor White
Write-Host "   Desktop: .\desktop\src-tauri\target\$TargetDir\" -ForegroundColor White
Write-Host ""
Write-Host "🎉 EzP3 is ready to use!" -ForegroundColor Green
Write-Host ""
Write-Host "Quick start:" -ForegroundColor Cyan
Write-Host "   CLI: .\dist\ezp3.exe convert 'https://youtube.com/watch?v=...' --format mp3" -ForegroundColor White
Write-Host "   Web: .\dist\ezp3-web-backend.exe" -ForegroundColor White
Write-Host "   Desktop: run the executable in desktop\src-tauri\target\$TargetDir\" -ForegroundColor White
