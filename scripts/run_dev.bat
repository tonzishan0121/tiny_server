@echo off
REM Start tiny_server in development mode (Windows).

cd /d "%~dp0.."

cargo build
if %errorlevel% neq 0 exit /b %errorlevel%

.\target\debug\tiny_server.exe
