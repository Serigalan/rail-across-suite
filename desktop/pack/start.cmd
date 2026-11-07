@echo off
rem Rail-Across-Suite, local package: starts the project server on 127.0.0.1 with a
rem free port, without user management, serving the built suite in app\ beside the
rem API — and opens the browser on it. The database lies in .\data\ beside this
rem script. Uses the bundled Node.js (node\bin\node.exe); only if that is
rem missing, Node.js 22 or later on PATH.
cd /d "%~dp0"
if not exist data mkdir data
set RA_SERVER_DB=data\rail-across.sqlite
set RA_SERVER_CLOUDS=data\clouds
if exist node\bin\node.exe (set "NODE=node\bin\node.exe") else (set "NODE=node")
start /b "%NODE%" server\bin\rail-across-server.mjs serve --local --static app --host 127.0.0.1 --port 0 > data\server.log 2>&1
set URL=
for /l %%i in (1,1,150) do (
  if not defined URL (
    for /f "tokens=1,* delims==" %%a in ('findstr /r "RA_DESKTOP_URL=.*" data\server.log 2^>nul') do set URL=%%b
  )
  if defined URL goto open
  timeout /t 1 /nobreak >nul
)
echo No address from the server — see data\server.log >&2
exit /b 1
:open
echo Rail-Across-Suite (local package) on %URL%
start "" "%URL%"
pause
