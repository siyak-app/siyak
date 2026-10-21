FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# System libraries required to build Tauri v2 on Linux.
# Keep in sync with CONTRIBUTING.md and .github/workflows/ci.yml.
RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        build-essential \
        curl \
        wget \
        file \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        libssl-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
