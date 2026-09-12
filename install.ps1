# irm https://github.com/scrtx/video-tools/releases/latest/download/install.ps1 | iex
$ErrorActionPreference = "Stop"
$repo = if ($env:NITRATE_REPO) { $env:NITRATE_REPO } else { "scrtx/video-tools" }
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
$asset = "nitrate-windows-$arch.exe"
$url = "https://github.com/$repo/releases/latest/download/$asset"
$dir = if ($env:NITRATE_INSTALL_DIR) { $env:NITRATE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "nitrate" }
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$dest = Join-Path $dir "nitrate.exe"
Write-Host "GET  $url"
Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
Write-Host "WRITE  $dest"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$dir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$dir", "User")
    $env:Path = "$env:Path;$dir"
    Write-Host "ADD    $dir to user PATH"
}
Write-Host "NITRATE  installed"
