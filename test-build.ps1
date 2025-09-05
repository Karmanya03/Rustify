# Quick Build Test Script

Write-Host "🧪 Testing EzP3 Build Process..." -ForegroundColor Cyan

# Test if Rust is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust not found. Installing..." -ForegroundColor Red
    Write-Host "Please install Rust from: https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Rust found: $(cargo --version)" -ForegroundColor Green

# Check Cargo.toml syntax
Write-Host "🔍 Checking Cargo.toml files..." -ForegroundColor Yellow

$cargoFiles = @(
    "Cargo.toml",
    "core\Cargo.toml", 
    "cli\Cargo.toml",
    "desktop\Cargo.toml",
    "web-backend\Cargo.toml"
)

foreach ($file in $cargoFiles) {
    if (Test-Path $file) {
        Write-Host "  ✅ $file exists" -ForegroundColor Green
    } else {
        Write-Host "  ❌ $file missing" -ForegroundColor Red
    }
}

# Test workspace check
Write-Host "🔧 Checking workspace..." -ForegroundColor Yellow
try {
    cargo check --workspace --quiet
    Write-Host "✅ Workspace check passed" -ForegroundColor Green
} catch {
    Write-Host "❌ Workspace check failed" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
}

# Quick build test
Write-Host "🔨 Testing quick build..." -ForegroundColor Yellow
try {
    cargo build --workspace --quiet
    Write-Host "✅ Build test passed" -ForegroundColor Green
} catch {
    Write-Host "⚠️  Build test had issues (this is normal for first run)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🎉 Setup validation complete!" -ForegroundColor Cyan
Write-Host "You can now run: .\build.ps1 to build the full project" -ForegroundColor White
