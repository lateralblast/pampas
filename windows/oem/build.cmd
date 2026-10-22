@echo off
setlocal
rem (C:\pampas\build.cmd in the VM is a stub that calls this file from the share, so edits here apply at once.)
rem Copies the repository out of the read-only share, builds the interface (Node), the GUI installer (NSIS) and the
rem pampas command-line binary, and copies them back to out\bundle (nsis\ and cli\). Arguments are ignored (kept for
rem the watcher's request file). Never run anything here that can prompt: the hidden watcher would wait for ever.
set SHARE=\\host.lan\Data
set LOG=%SHARE%\out\build.log
set WORK=C:\pampas\work
set RUSTUP_HOME=C:\pampas\rustup
set CARGO_HOME=C:\pampas\cargo
rem tauri's Windows resource step needs RC.EXE (Windows SDK) on the PATH; the newest SDK's x64 tools folder is used
rem (calling vcvars64.bat from here ended the script early, and cargo finds the MSVC linker by itself).
rem Never use RC as a variable name: embed-resource reads it as the path of rc.exe.
for /d %%d in ("C:\Program Files (x86)\Windows Kits\10\bin\10.*") do if exist "%%d\x64\rc.exe" set "SDKBIN=%%d\x64"
set PATH=%SDKBIN%;C:\pampas\cargo\bin;C:\pampas\node-v22.14.0-win-x64;C:\pampas\nasm-2.16.03;C:\pampas\cmake-3.31.6-windows-x86_64\bin;%PATH%
del "%SHARE%\out\build.done" 2>nul
del "%SHARE%\out\build.log" 2>nul
rem Self-healing: installs any build tool that is missing (Node, NASM, CMake, tauri-cli); instant when they are all there
echo Checking the build tools >> "%LOG%"
copy /Y "%SHARE%\out\oem\setup.cmd" C:\pampas\setup.cmd >nul
call C:\pampas\setup.cmd >> "%LOG%" 2>&1
rmdir /S /Q "%SHARE%\out\bundle" 2>nul
echo Copying the repository >> "%LOG%"
rem /XD keeps target\ and node_modules\ between builds (excluded folders are not deleted from the copy), so only the first build compiles everything
robocopy "%SHARE%\src" %WORK% /MIR /XD "%SHARE%\src\windows\out" "%SHARE%\src\dist" "%SHARE%\src\target" "%SHARE%\src\.git" "%SHARE%\src\.impeccable" "%SHARE%\src\ui\node_modules" "%SHARE%\src\ui\dist" "%SHARE%\src\src-tauri\gen" %WORK%\target %WORK%\ui\node_modules /XF *.pyc /NFL /NDL /NJH /NJS /NP >> "%LOG%" 2>&1
rem robocopy exits with 1 when it copied files, which is success; clear it so the checks below only see what they ran
ver >nul
set BUILDRC=0
echo Building the interface >> "%LOG%"
cd /d %WORK%\ui
if exist node_modules\.bin\tsc.cmd goto npm_ready
call npm ci < nul >> "%LOG%" 2>&1
if errorlevel 1 set BUILDRC=1
:npm_ready
if "%BUILDRC%"=="0" call npm run build < nul >> "%LOG%" 2>&1
if errorlevel 1 set BUILDRC=1
if "%BUILDRC%"=="0" (
  cd /d %WORK%\src-tauri
  echo Building the GUI installer >> "%LOG%"
  rem installers from earlier versions stay in target\ between builds and would be copied out too
  del /Q %WORK%\target\release\bundle\nsis\*-setup.exe 2>nul
  cargo tauri build --bundles nsis < nul >> "%LOG%" 2>&1
  if errorlevel 1 set BUILDRC=1
)
rem the installer is copied as soon as it exists, so a failing command-line build does not lose it
if "%BUILDRC%"=="0" xcopy /E /Y /I /Q %WORK%\target\release\bundle\nsis "%SHARE%\out\bundle\nsis" >> "%LOG%" 2>&1
if "%BUILDRC%"=="0" (
  cd /d %WORK%
  echo Building the pampas command-line binary >> "%LOG%"
  cargo build --release -p pampas < nul >> "%LOG%" 2>&1
  if errorlevel 1 set BUILDRC=1
)
rem plain copy into a directory that exists: xcopy asked "file or directory?" for a single file and waited for ever
if "%BUILDRC%"=="0" mkdir "%SHARE%\out\bundle\cli" 2>nul
if "%BUILDRC%"=="0" copy /Y %WORK%\target\release\pampas.exe "%SHARE%\out\bundle\cli\" >> "%LOG%" 2>&1
>"%SHARE%\out\build.done" echo %BUILDRC%
