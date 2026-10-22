#!/bin/sh
# Runs inside the docker/windows image (started by `./build.sh --target windows --docker`): builds the
# NSIS installer and pampas.exe for x86_64-pc-windows-gnu into /src/windows/out/cross.
set -eu
T=x86_64-pc-windows-gnu
out=/src/windows/out/cross
rm -rf "$out"; mkdir -p "$out"
cd /src
# the interface, which the app embeds (a volume is mounted on ui/node_modules, so test for the tool, not the folder)
[ -x ui/node_modules/.bin/tsc ] || npm --prefix ui ci
npm --prefix ui run build
cd /src/src-tauri
cargo tauri build --target $T --bundles nsis
cp /src/target/$T/release/bundle/nsis/*-setup.exe "$out/"
if [ "${1:-}" != --no-cli ]; then
  cd /src
  cargo build --release -p pampas --target $T
  cp target/$T/release/pampas.exe "$out/"
fi
