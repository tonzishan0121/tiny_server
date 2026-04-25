@echo off
REM Start tiny_server in release mode (Windows).

cd /d "%~dp0.."

cargo build --release
if %errorlevel% neq 0 exit /b %errorlevel%

.\target\release\tiny_server.exe
