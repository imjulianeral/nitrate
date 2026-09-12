# irm https://github.com/imjulianeral/nitrate/releases/latest/download/install.ps1 | iex
$ErrorActionPreference = "Stop"
$repo = if ($env:NITRATE_REPO) { $env:NITRATE_REPO } else { "imjulianeral/nitrate" }
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
$asset = "nitrate-windows-$arch.zip"
$url = "https://github.com/$repo/releases/latest/download/$asset"
$dir = if ($env:NITRATE_INSTALL_DIR) { $env:NITRATE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "nitrate" }
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$zip = Join-Path $env:TEMP "nitrate-install-$PID.zip"
$stage = Join-Path $env:TEMP "nitrate-install-$PID"
Write-Host "GET  $url"
Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Expand-Archive -Path $zip -DestinationPath $stage -Force
$src = Get-ChildItem -Path $stage -Filter "nitrate.exe" -Recurse | Select-Object -First 1
if (-not $src) { throw "archive is missing nitrate.exe" }
Copy-Item $src.FullName (Join-Path $dir "nitrate.exe") -Force
$tools = Join-Path $src.DirectoryName "tools"
if (Test-Path $tools) {
    $destTools = Join-Path $dir "tools"
    New-Item -ItemType Directory -Force -Path $destTools | Out-Null
    Copy-Item (Join-Path $tools "*") $destTools -Force
}
Remove-Item -Force $zip
Remove-Item -Recurse -Force $stage
Write-Host "WRITE  $(Join-Path $dir 'nitrate.exe')"
Write-Host "WRITE  $(Join-Path $dir 'tools')"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$dir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$dir", "User")
    $env:Path = "$env:Path;$dir"
    Write-Host "ADD    $dir to user PATH"
}
Write-Host "NITRATE  installed"
