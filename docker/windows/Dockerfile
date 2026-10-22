# Build image for `./build.sh --target windows --docker`: cross-compiles the Windows installer and
# pampas-cli.exe (x86_64-pc-windows-gnu, MinGW) in a Linux container, so a Mac can produce them.
# Nothing here is Windows software; run it natively (no --platform) on both Intel and Apple Silicon.
FROM ubuntu:24.04
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential pkg-config curl wget ca-certificates gnupg git file perl make xz-utils cmake nasm \
      mingw-w64 nsis \
    && update-alternatives --set x86_64-w64-mingw32-gcc /usr/bin/x86_64-w64-mingw32-gcc-posix \
    && update-alternatives --set x86_64-w64-mingw32-g++ /usr/bin/x86_64-w64-mingw32-g++-posix \
    && rm -rf /var/lib/apt/lists/*
# Node builds the interface (ui/)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable -t x86_64-pc-windows-gnu \
    && cargo install tauri-cli --version '^2' --locked
WORKDIR /src
