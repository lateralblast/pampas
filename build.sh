#!/usr/bin/env bash
# Build pampas installers (the desktop app) and the command-line binary.
#   ./build.sh --target mac [--arch arm64|x64|universal]                     (on a Mac: builds a .dmg; untested, see TODO.md)
#   ./build.sh --target linux [--package deb|rpm|appimage|all]               (comma list allowed, e.g. deb,rpm; default: the
#                                                                 distribution's own package: deb on Debian/Ubuntu, rpm on
#                                                                 RHEL/Fedora/Alma/Rocky/SUSE, else appimage;
#                                                                 --target deb / --target rpm = linux with that package only)
#   ./build.sh --target linux --docker [--arch x64|arm64] [--package ...]    (Linux packages in a Docker container, docker/linux/Dockerfile,
#                                                                 Ubuntu 24.04; --arch picks the container platform, default
#                                                                 x64 = linux/amd64; the default for --target linux on a host that
#                                                                 is not Linux, e.g. a Mac)
#   ./build.sh --target windows      (on Linux: builds the installer in a Windows 11 VM run by Docker, windows/vm.py; needs docker and
#                                     /dev/kvm, and `python3 windows/vm.py up` once first; on Windows, e.g. in Git Bash: builds natively)
#   ./build.sh --target windows --docker   (cross-compiles in a Linux container, docker/windows/Dockerfile: MinGW,
#                                     x86_64-pc-windows-gnu; the default on a Mac; nothing built this way has been run on Windows)
# The app's installer and the command line (pampas-cli) are built apart: the installer is a .deb/.rpm/AppImage, .exe or .dmg, the
# command line an archive, pampas-cli-<version>-<system>.tar.gz or .zip. Add --no-cli to skip the command line.
# Results are copied to ./dist/ (gitignored).
# A target can only be built on its own platform (the app links the system web view), except through the VM and Docker routes above.
# Needs the Rust toolchain, Node (the interface is built with it) and tauri-cli (cargo install tauri-cli --version '^2').
# It never commits, tags, pushes or publishes anything.
set -euo pipefail

usage() {
  echo "usage: $0 --target mac|linux|deb|rpm|windows [--arch arm64|x64|universal] [--package deb|rpm|appimage|all] [--docker] [--no-cli]"
  exit "${1:-1}"
}

target="" arch="" package="" docker=0 cli=1
while [ $# -gt 0 ]; do
  case "$1" in
    --target) target="${2:-}"; shift 2 ;;
    --arch) arch="${2:-}"; shift 2 ;;
    --package|--format) package="${2:-}"; shift 2 ;;
    --docker) docker=1; shift ;;
    --no-cli) cli=0; shift ;;
    -h|--help) usage 0 ;;
    *) echo "unknown option: $1" >&2; usage ;;
  esac
done
[ -n "$target" ] || usage

root=$(cd "$(dirname "$0")" && pwd)
dist="$root/dist"
# the version of the root package (the first `version =` line of Cargo.toml, under [package])
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)

# The package format of the Linux distribution this runs on (/etc/os-release ID and ID_LIKE).
native_package() {
  local ids=""
  [ -r /etc/os-release ] && ids=$(. /etc/os-release; echo "${ID:-} ${ID_LIKE:-}")
  case " $ids " in
    *" debian "*|*" ubuntu "*) echo deb ;;
    *" rhel "*|*" fedora "*|*" centos "*|*" almalinux "*|*" rocky "*|*" suse "*|*" opensuse "*|*" opensuse-leap "*|*" sles "*|*" amzn "*|*" ol "*) echo rpm ;;
    *) echo appimage ;;
  esac
}

# The interface (ui/) is plain TypeScript compiled by Node into ui/dist, which the app embeds. `cargo tauri build` runs
# this too (beforeBuildCommand); doing it first here gives a clear error, and a plain build with no front end would
# otherwise embed a placeholder page.
build_ui() {
  command -v npm >/dev/null || { echo "npm not found: the interface is built with Node (https://nodejs.org, version 20 or later)" >&2; exit 1; }
  echo "== building the interface =="
  # the tsc check, not just the folder: a Docker volume mounted on node_modules is an empty folder
  [ -x "$root/ui/node_modules/.bin/tsc" ] || npm --prefix "$root/ui" ci
  npm --prefix "$root/ui" run build
}

# The command line is shipped as an archive, apart from the app's installer: pampas-cli-<version>-<label>.tar.gz
# (Linux, macOS) or .zip (Windows), holding a folder of the same name with just the binary in it.
# usage: package_cli <path to the built binary> <label, e.g. linux or windows> [.exe]
package_cli() {
  local bin="$1" label="$2" exe="${3:-}" name work
  name="pampas-cli-$version-$label"
  work=$(mktemp -d)
  mkdir "$work/$name"
  cp "$bin" "$work/$name/pampas-cli$exe"
  chmod +x "$work/$name/pampas-cli$exe"
  mkdir -p "$dist"
  if [ -n "$exe" ]; then
    rm -f "$dist/$name.zip"
    if command -v zip >/dev/null; then
      (cd "$work" && zip -q -r "$dist/$name.zip" "$name")
    elif command -v python3 >/dev/null; then
      python3 -I -c 'import shutil, sys; shutil.make_archive(sys.argv[1], "zip", sys.argv[2], sys.argv[3])' "$dist/$name" "$work" "$name"
    else
      echo "neither zip nor python3 found: cannot make $name.zip" >&2; exit 1
    fi
    echo "  $dist/$name.zip"
  else
    (cd "$work" && tar -czf "$dist/$name.tar.gz" "$name")
    echo "  $dist/$name.tar.gz"
  fi
  rm -rf "$work"
}

# Windows from a Linux host: the installer and the command-line binary are built in the Windows 11 VM (windows/vm.py)
windows_vm() {
  command -v python3 >/dev/null || { echo "python3 not found (windows/vm.py needs it)" >&2; exit 1; }
  vmrc=0
  python3 -I "$root/windows/vm.py" build --manage || vmrc=$?
  mkdir -p "$dist"
  found=0
  for f in "$root"/windows/out/bundle/nsis/*-setup.exe; do
    [ -f "$f" ] || continue
    cp "$f" "$dist/"; echo "  $dist/$(basename "$f")"; found=1
  done
  [ "$found" = 1 ] || { echo "no installer found under windows/out/bundle/nsis" >&2; exit 1; }
  if [ "$cli" = 1 ] && [ -f "$root/windows/out/bundle/cli/pampas-cli.exe" ]; then
    package_cli "$root/windows/out/bundle/cli/pampas-cli.exe" windows .exe
  fi
  if [ "$vmrc" != 0 ]; then
    echo "the Windows build failed (see windows/out/build.log); the files above are what was built before it failed" >&2
    exit "$vmrc"
  fi
  echo "done: $dist"
}

# Linux packages in a container (a Mac, or any host with Docker). The repo is mounted at /src; the target directory,
# node_modules and the cargo registry live in Docker volumes so they never mix with the host's own builds; `build.sh`
# runs inside on Linux and writes to dist/.
linux_docker() {
  command -v docker >/dev/null || { echo "docker not found (needed for --docker, and for --target linux on a host that is not Linux)" >&2; exit 1; }
  docker info >/dev/null 2>&1 || { echo "the Docker daemon is not running" >&2; exit 1; }
  local plat tag
  case "${arch:-x64}" in
    x64|amd64|x86_64) plat=linux/amd64; tag=amd64 ;;
    arm64|aarch64) plat=linux/arm64; tag=arm64 ;;
    *) echo "unknown --arch for Linux: $arch (x64, arm64)" >&2; exit 1 ;;
  esac
  echo "== Linux build in Docker ($plat) =="
  docker build --platform "$plat" -t "pampas-linux-build:$tag" "$root/docker/linux"
  mkdir -p "$dist"
  local inner=(./build.sh --target linux)
  [ -z "$package" ] || inner+=(--package "$package")
  [ "$cli" = 1 ] || inner+=(--no-cli)
  docker run --rm --platform "$plat" \
    -v "$root":/src \
    -v "pampas-target-$tag":/src/target \
    -v "pampas-ui-modules-$tag":/src/ui/node_modules \
    -v "pampas-cargo-$tag":/opt/cargo/registry \
    -e PAMPAS_LINUX_ARCH="$tag" -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
    "pampas-linux-build:$tag" \
    sh -c '"$@"; rc=$?; chown -R "$HOST_UID:$HOST_GID" /src/dist /src/Cargo.lock /src/ui/dist /src/ui/package-lock.json /src/src-tauri/gen 2>/dev/null; exit $rc' sh "${inner[@]}"
  echo "done: $dist"
}

# Windows installer and command-line binary cross-compiled in a Linux container (a Mac, or Linux without the VM).
windows_docker() {
  command -v docker >/dev/null || { echo "docker not found" >&2; exit 1; }
  docker info >/dev/null 2>&1 || { echo "the Docker daemon is not running" >&2; exit 1; }
  echo "== Windows cross build in Docker (x86_64-pc-windows-gnu) =="
  docker build -t pampas-windows-cross "$root/docker/windows"
  mkdir -p "$dist"
  local rc=0 inner=()
  [ "$cli" = 1 ] || inner+=(--no-cli)
  docker run --rm \
    -v "$root":/src \
    -v pampas-wincross-target:/src/target \
    -v pampas-wincross-ui-modules:/src/ui/node_modules \
    -v pampas-wincross-cargo:/opt/cargo/registry \
    -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
    pampas-windows-cross \
    sh -c '/src/docker/windows/build.sh "$@"; rc=$?; chown -R "$HOST_UID:$HOST_GID" /src/windows/out/cross /src/Cargo.lock /src/ui/dist /src/ui/package-lock.json /src/src-tauri/gen 2>/dev/null; exit $rc' sh ${inner[@]+"${inner[@]}"} || rc=$?
  [ "$rc" = 0 ] || { echo "the Windows cross build failed" >&2; exit "$rc"; }
  local f
  for f in "$root"/windows/out/cross/*-setup.exe; do
    [ -f "$f" ] || continue
    cp "$f" "$dist/pampas_${version}_x64-setup.exe"; echo "  $dist/pampas_${version}_x64-setup.exe"
  done
  if [ "$cli" = 1 ] && [ -f "$root/windows/out/cross/pampas-cli.exe" ]; then
    package_cli "$root/windows/out/cross/pampas-cli.exe" windows .exe
  fi
  echo "done: $dist"
}

build_target() {
  host=$(uname -s)
  case "$target" in deb|rpm) package="$target"; target=linux ;; esac
  case "$target" in
    mac|macos)   want=Darwin; bundles=dmg ;;
    linux)       want=Linux;  bundles=$(native_package) ;;
    windows|win) want=MINGW;  bundles=nsis ;;
    *) echo "unknown target: $target (mac, linux, deb, rpm, windows)" >&2; exit 1 ;;
  esac
  if [ -n "$package" ] && [ "$target" != linux ]; then
    echo "--package only applies to --target linux" >&2; exit 1
  fi
  if [ "$target" = linux ] && [ -n "$package" ]; then
    bundles=""
    for p in ${package//,/ }; do
      case "$p" in
        all) bundles="deb,rpm,appimage"; break ;;
        deb|rpm|appimage) bundles="${bundles:+$bundles,}$p" ;;
        *) echo "unknown --package: $p (deb, rpm, appimage, all)" >&2; exit 1 ;;
      esac
    done
  fi
  if [ "$target" = linux ] && { [ "$docker" = 1 ] || [ "$host" != Linux ]; }; then
    linux_docker; return
  fi
  if [ "$target" = linux ]; then
    case ",$bundles," in
      *,rpm,*) command -v rpmbuild >/dev/null || { echo "rpmbuild not found: install it (Debian/Ubuntu: apt install rpm; Fedora: dnf install rpm-build)" >&2; exit 1; } ;;
    esac
  fi
  if [ "$target" = windows ] || [ "$target" = win ]; then
    if [ "$docker" = 1 ] || [ "$host" = Darwin ]; then windows_docker; return; fi
    if [ "$host" = Linux ]; then windows_vm; return; fi
  fi
  case "$host" in
    $want*) ;;
    *) echo "the $target installer must be built on $target (this is $host)" >&2; exit 1 ;;
  esac

  command -v cargo >/dev/null || { echo "cargo not found: install Rust (https://rustup.rs)" >&2; exit 1; }
  cargo tauri --version >/dev/null 2>&1 || { echo "tauri-cli not found: cargo install tauri-cli --version '^2'" >&2; exit 1; }

  triple="" triple_args=()
  if [ "$target" = mac ] || [ "$target" = macos ]; then
    case "${arch:-native}" in
      native) ;;
      arm64) triple=aarch64-apple-darwin ;;
      x64) triple=x86_64-apple-darwin ;;
      universal) triple=universal-apple-darwin ;;
      *) echo "unknown --arch: $arch (arm64, x64, universal)" >&2; exit 1 ;;
    esac
    if [ -n "$triple" ]; then
      triple_args=(--target "$triple")
      # make sure the Rust standard library for the requested architecture is installed
      case "$triple" in
        universal-apple-darwin) rustup target add aarch64-apple-darwin x86_64-apple-darwin ;;
        *) rustup target add "$triple" ;;
      esac
    fi
  elif [ -n "$arch" ]; then
    echo "--arch only applies to --target mac (or --target linux with --docker)" >&2; exit 1
  fi
  [ "$docker" = 0 ] || [ "$target" = linux ] || [ "$target" = windows ] || { echo "--docker only applies to --target linux and windows" >&2; exit 1; }

  mkdir -p "$dist"
  build_ui
  stamp=$(mktemp)
  echo "== building the installer ($target, $bundles) =="
  # cargo tauri build runs in src-tauri; the workspace puts the bundles under the root target/ directory
  (cd "$root/src-tauri" && cargo tauri build --bundles "$bundles" ${triple_args[@]+"${triple_args[@]}"})

  # Tauri puts bundles under target/[<triple>/]release/bundle/<kind>/
  found=0
  while IFS= read -r f; do
    cp -R "$f" "$dist/"
    echo "  $dist/$(basename "$f")"
    found=1
  done < <(find "$root/target" -path '*/release/bundle/*' -type f \
    \( -name '*.dmg' -o -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' -o -name '*-setup.exe' \) -newer "$stamp" 2>/dev/null)
  rm -f "$stamp"
  [ "$found" = 1 ] || { echo "no installer found under target/ (did the bundle step run?)" >&2; exit 1; }

  if [ "$cli" = 1 ]; then
    echo "== building the pampas-cli command-line binary =="
    exe=""; [ "$target" != windows ] || exe=".exe"
    if [ "$triple" = universal-apple-darwin ]; then
      # plain cargo has no universal target: build both architectures and join them with lipo
      (cd "$root" && cargo build --release -p pampas --target aarch64-apple-darwin &&
        cargo build --release -p pampas --target x86_64-apple-darwin)
      mkdir -p "$root/target/universal-apple-darwin/release"
      lipo -create -output "$root/target/universal-apple-darwin/release/pampas-cli" \
        "$root/target/aarch64-apple-darwin/release/pampas-cli" "$root/target/x86_64-apple-darwin/release/pampas-cli"
      bin="$root/target/universal-apple-darwin/release/pampas-cli"
    elif [ -n "$triple" ]; then
      (cd "$root" && cargo build --release -p pampas --target "$triple")
      bin="$root/target/$triple/release/pampas-cli$exe"
    else
      (cd "$root" && cargo build --release -p pampas)
      bin="$root/target/release/pampas-cli$exe"
    fi
    [ -f "$bin" ] || { echo "the command-line binary was not found at $bin" >&2; exit 1; }
    # the archive's label: the system and, on a Mac, the architecture; an arm64 Docker build is named apart from the x64 one
    label="$target"
    case "$target" in
      mac|macos)
        case "${triple:-native}" in
          aarch64-apple-darwin) label=mac-arm64 ;;
          x86_64-apple-darwin) label=mac-x64 ;;
          universal-apple-darwin) label=mac-universal ;;
          *) case "$(uname -m)" in arm64|aarch64) label=mac-arm64 ;; *) label=mac-x64 ;; esac ;;
        esac ;;
      linux) [ "${PAMPAS_LINUX_ARCH:-}" != arm64 ] || label=linux-arm64 ;;
    esac
    package_cli "$bin" "$label" "$exe"
  fi
  echo "done: $dist"
}

build_target
