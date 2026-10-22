@echo off
rem Waits for out\build.request on the host share, then runs build.cmd with the arguments in that file.
set OUT=\\host.lan\Data\out
:loop
if exist "%OUT%\build.request" call :request
timeout /t 5 /nobreak >nul
goto loop

rem A subroutine, because %ARGS% inside a parenthesised block would be expanded before set /p ran
:request
rem setlocal: whatever build.cmd sets (it once set OUT, which made the watcher poll the wrong folder for ever) is dropped afterwards
setlocal
set /p ARGS=<"%OUT%\build.request"
del "%OUT%\build.request"
call C:\pampas\build.cmd %ARGS%
endlocal
goto :eof
