FROM mcr.microsoft.com/devcontainers/typescript-node

# Install Tauri system dependencies
# Based on: https://v2.tauri.app/start/prerequisites/
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    pango1.0-dev \
    libgdk-pixbuf-xlib-2.0-dev \
    xdg-utils \
    neovim \
    ripgrep \
    fd-find \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*