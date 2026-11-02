@echo off
rem The Layout-Tool, local version: serves app\ on http://localhost:8080/ (see README.md).
cd /d "%~dp0"
node serve.mjs %*
pause
