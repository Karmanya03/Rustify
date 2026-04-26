# setup-windows.ps1 - Auto Setup for Rustify on Windows

$dl = Join-Path $HOME "Downloads\Rustify"
New-Item -ItemType Directory -Force -Path $dl | Out-Null

$yt = (Get-Command yt-dlp -ErrorAction SilentlyContinue).Source
$ff = (Get-Command ffmpeg -ErrorAction SilentlyContinue).Source

$browser = if (Test-Path "$env:ProgramFiles\Google\Chrome\Application\chrome.exe") { 
    "chrome" 
} elseif (Test-Path "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe") { 
    "edge" 
} elseif (Test-Path "$env:ProgramFiles\Mozilla Firefox\firefox.exe") { 
    "firefox" 
} else { 
    "edge" 
}

rustify config reset
rustify config set download_dir "$dl"
rustify config set auth.mode auto
rustify config set auth.browser $browser

if ($yt) { rustify config set binaries.yt_dlp "$yt" }
if ($ff) { rustify config set binaries.ffmpeg "$ff" }

rustify config show
