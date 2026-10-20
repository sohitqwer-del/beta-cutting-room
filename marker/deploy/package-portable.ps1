# Package existing, verified outputs. Does not compile or publish anything.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Cli,
    [Parameter(Mandatory = $true)][string]$WebDir,
    [Parameter(Mandatory = $true)][string]$OutputDir
)
$ErrorActionPreference = 'Stop'
$markerRoot = Split-Path $PSScriptRoot -Parent
$repoRoot = Split-Path $markerRoot -Parent
$version = (Get-Content (Join-Path $markerRoot 'apps/desktop/src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json).version
$cliPath = (Resolve-Path -LiteralPath $Cli).Path
$webPath = (Resolve-Path -LiteralPath $WebDir).Path
foreach ($name in @('index.html', 'wasm/beta_marker_wasm_bg.wasm', 'wasm/beta_pattern_wasm_bg.wasm', 'wasm/beta_marker_floor_bg.wasm')) {
    if (-not (Test-Path -LiteralPath (Join-Path $webPath $name) -PathType Leaf)) { throw "Missing web release asset: $name" }
}
$outputRoot = [IO.Path]::GetFullPath($OutputDir)
[IO.Directory]::CreateDirectory($outputRoot) | Out-Null
$stageRoot = Join-Path $outputRoot ('.staging-' + [guid]::NewGuid().ToString('N'))
$stage = Join-Path $stageRoot "BETA-Cutting-Room_${version}_windows-x64-portable"
[IO.Directory]::CreateDirectory($stage) | Out-Null
Copy-Item -LiteralPath $cliPath -Destination (Join-Path $stage 'beta-marker.exe')
Copy-Item -LiteralPath $webPath -Destination (Join-Path $stage 'web') -Recurse
foreach ($name in @('README.md', 'CHANGELOG.md', 'NOTICE', 'THIRD-PARTY-LICENSES.md', 'SECURITY.md')) {
    Copy-Item -LiteralPath (Join-Path $markerRoot $name) -Destination $stage
}
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination $stage
$docStage = Join-Path $stage 'docs'
[IO.Directory]::CreateDirectory($docStage) | Out-Null
foreach ($doc in Get-ChildItem -LiteralPath (Join-Path $markerRoot 'docs')) {
    # Keep the artifact hash record outside its own archive (no self-referencing hash).
    if ($doc.Name -ne 'RELEASE-0.4.0.md') { Copy-Item -LiteralPath $doc.FullName -Destination $docStage -Recurse }
}
Copy-Item -LiteralPath (Join-Path $markerRoot 'deploy') -Destination $stage -Recurse
$notices = Join-Path $outputRoot 'THIRD-PARTY-NOTICES.txt'
if (-not (Test-Path -LiteralPath $notices -PathType Leaf)) { throw 'Generate THIRD-PARTY-NOTICES.txt with generate-licenses.py --texts first.' }
Copy-Item -LiteralPath $notices -Destination $stage
@'
@echo off
cd /d "%~dp0"
beta-marker.exe serve --web-dir web %*
'@ | Set-Content -LiteralPath (Join-Path $stage 'start-server.cmd') -Encoding ascii
$zip = Join-Path $outputRoot "BETA-Cutting-Room_${version}_windows-x64-portable.zip"
if (Test-Path -LiteralPath $zip) { throw "Output already exists: $zip" }
Compress-Archive -LiteralPath $stage -DestinationPath $zip -CompressionLevel Optimal
$file = Get-Item -LiteralPath $zip
$hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
[pscustomobject]@{ Path = $file.FullName; Bytes = $file.Length; SHA256 = $hash; StagingDirectory = $stage }
