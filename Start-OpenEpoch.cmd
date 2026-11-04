@echo off
cd /d "%~dp0"
set "OPEN_EPOCH_PYTHON=python"
if exist "C:\Python314\python.exe" set "OPEN_EPOCH_PYTHON=C:\Python314\python.exe"
"%OPEN_EPOCH_PYTHON%" "tools\serve_game.py" %*
if errorlevel 1 pause
