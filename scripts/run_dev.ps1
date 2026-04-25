# Start tiny_server in development mode (Windows PowerShell).
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ProjectDir = Split-Path -Parent $PSScriptRoot

Set-Location $ProjectDir
cargo build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& ".\target\debug\tiny_server.exe"
exit $LASTEXITCODE
