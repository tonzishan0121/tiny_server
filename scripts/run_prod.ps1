# Start tiny_server in release mode (Windows PowerShell).
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ProjectDir = Split-Path -Parent $PSScriptRoot

Set-Location $ProjectDir
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& ".\target\release\tiny_server.exe"
exit $LASTEXITCODE
