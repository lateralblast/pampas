@echo off
rem Installs what is still missing after Visual Studio Build Tools and Rust: Node (builds the interface in ui\), NASM and
rem CMake (the rustls crypto library, aws-lc-sys, needs them on Windows) and tauri-cli. All are plain zip downloads
rem unpacked with the tar that ships with Windows, so nothing here asks questions. Safe to run again; install.bat
rem calls it, and it can be run by hand in the VM.
set ROOT=C:\pampas
set LOG=%ROOT%\install.log
set RUSTUP_HOME=%ROOT%\rustup
set CARGO_HOME=%ROOT%\cargo
set NODE=%ROOT%\node-v22.14.0-win-x64
set NASM=%ROOT%\nasm-2.16.03
set CMAKE=%ROOT%\cmake-3.31.6-windows-x86_64\bin
set PATH=%CARGO_HOME%\bin;%NODE%;%NASM%;%CMAKE%;%PATH%
mkdir %ROOT%\dl 2>nul

if exist %NODE%\node.exe goto node_done
echo Installing Node >> %LOG%
curl.exe -L -o %ROOT%\dl\node.zip https://nodejs.org/dist/v22.14.0/node-v22.14.0-win-x64.zip >> %LOG% 2>&1
tar -xf %ROOT%\dl\node.zip -C %ROOT% >> %LOG% 2>&1
:node_done

if exist %NASM%\nasm.exe goto nasm_done
echo Installing NASM >> %LOG%
curl.exe -L -o %ROOT%\dl\nasm.zip https://www.nasm.us/pub/nasm/releasebuilds/2.16.03/win64/nasm-2.16.03-win64.zip >> %LOG% 2>&1
tar -xf %ROOT%\dl\nasm.zip -C %ROOT% >> %LOG% 2>&1
:nasm_done

if exist %CMAKE%\cmake.exe goto cmake_done
echo Installing CMake >> %LOG%
curl.exe -L -o %ROOT%\dl\cmake.zip https://github.com/Kitware/CMake/releases/download/v3.31.6/cmake-3.31.6-windows-x86_64.zip >> %LOG% 2>&1
tar -xf %ROOT%\dl\cmake.zip -C %ROOT% >> %LOG% 2>&1
:cmake_done

if exist %CARGO_HOME%\bin\cargo-tauri.exe goto tauri_done
echo Installing tauri-cli >> %LOG%
cargo install tauri-cli --version "^2" --locked >> %LOG% 2>&1
:tauri_done

copy /Y %LOG% \\host.lan\Data\out\install.log >nul 2>&1
