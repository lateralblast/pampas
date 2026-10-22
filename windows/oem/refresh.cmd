@echo off
rem Installs the current oem scripts from the share into C:\pampas and restarts the watcher (for when they change after install).
set SRC=\\host.lan\Data\out\oem
copy /Y %SRC%\watch.cmd C:\pampas\watch.cmd
rem build.cmd in C:\pampas is a stub (stub.cmd) that calls the current one from the share
copy /Y %SRC%\stub.cmd C:\pampas\build.cmd
copy /Y %SRC%\watch.cmd "C:\ProgramData\Microsoft\Windows\Start Menu\Programs\StartUp\pampas-watch.cmd"
taskkill /F /FI "WINDOWTITLE eq pampas-watch*"
taskkill /F /FI "WINDOWTITLE eq C:\pampas\watch.cmd*"
start "pampas-watch" /MIN cmd /c C:\pampas\watch.cmd
