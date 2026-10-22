@echo off
rem Installed as C:\pampas\build.cmd: runs the current build.cmd that vm.py staged on the share.
call \\host.lan\Data\out\oem\build.cmd %*
