# Use Microsoft's Ubuntu devcontainer base
# See: https://github.com/microsoft/vscode-dev-containers/tree/main/containers/ubuntu
ARG VARIANT="jammy"
FROM mcr.microsoft.com/devcontainers/base:${VARIANT}

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
    libgdk-pixbuf2.0-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install pnpm globally (as vscode user after switching)
USER vscode
RUN curl -fsSL https://get.pnpm.io/install.sh | sh -

# Add pnpm to PATH
ENV PNPM_HOME="/home/vscode/.local/share/pnpm"
ENV PATH="${PNPM_HOME}:${PATH}"

# Switch back to root for any remaining setup
USER root
