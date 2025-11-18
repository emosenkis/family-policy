# Development Container Configuration

This directory contains the configuration for developing the Family Policy application in a containerized environment using VS Code Dev Containers.

## What's Included

### Base Environment
- **Ubuntu 22.04 (Jammy)** - Minimal Microsoft devcontainer base
- **Rust** - Latest stable toolchain with default profile
- **Node.js** - LTS version
- **pnpm** - Fast, disk space efficient package manager

### System Dependencies
All Tauri v2 prerequisites are pre-installed:
- Build essentials (gcc, make, etc.)
- GTK 3 development libraries
- WebKit2GTK 4.1 for web rendering
- AppIndicator support for system tray
- librsvg2 for SVG rendering
- Additional graphics libraries (Pango, GDK-Pixbuf)

### VS Code Extensions
Pre-configured extensions for optimal development:
- **Rust**: rust-analyzer, Even Better TOML, Crates, LLDB debugger
- **Vue**: Volar, TypeScript Vue Plugin
- **General**: ESLint, Prettier

## Usage

### Prerequisites
- [Docker Desktop](https://www.docker.com/products/docker-desktop)
- [VS Code](https://code.visualstudio.com/) with [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

### Getting Started

1. **Open in Container**
   - Open this repository in VS Code
   - Press `F1` and select "Dev Containers: Reopen in Container"
   - Wait for the container to build (first time takes 5-10 minutes)

2. **Verify Setup**
   ```bash
   rustc --version
   cargo --version
   node --version
   pnpm --version
   ```

3. **Build the Project**
   ```bash
   # Build CLI only
   cargo build --release

   # Build with UI
   cargo build --release --features ui
   ```

4. **Run the Application**
   ```bash
   # Note: GUI applications require X11 forwarding or similar
   # See "GUI Applications" section below
   cargo run -- --help
   ```

## Performance Optimization

The devcontainer is configured with volume mounts for:
- **Cargo cache** (`.devcontainer/cache/cargo`) - Stores downloaded crates
- **Build artifacts** (`.devcontainer/cache/target`) - Stores compiled binaries

This significantly improves build performance compared to host filesystem sharing.

### First Build Cache
On first build, Cargo will download and compile dependencies. Subsequent builds will be much faster due to cached artifacts.

## GUI Applications

Running Tauri UI mode in a container requires display forwarding:

### Linux/macOS with X11
```bash
# On host, allow container to connect to X server
xhost +local:docker

# In container, DISPLAY is already set
cargo run --features ui -- ui
```

### Windows with WSL2
Use WSLg (built into Windows 11) or install an X server like VcXsrv.

### Alternative: Use Host Build
For full GUI development, consider:
- Building CLI in container for consistent environment
- Building UI on host for native display integration

## Troubleshooting

### Container Build Fails
- Ensure Docker Desktop is running
- Check available disk space (need ~5GB)
- Try rebuilding: `F1` → "Dev Containers: Rebuild Container"

### Slow Builds
- First build is always slow (downloading dependencies)
- Ensure volume mounts are working (check `.devcontainer/cache/` exists)
- On Windows, use WSL2 backend for better performance

### Rust-analyzer Issues
- Wait for initial indexing to complete (check bottom-right status)
- Reload window: `F1` → "Developer: Reload Window"
- Rebuild container if issues persist

## Customization

### Change Ubuntu Version
Edit `devcontainer.json`:
```json
"args": {
  "VARIANT": "focal"  // or "bionic"
}
```

### Add VS Code Extensions
Add to `customizations.vscode.extensions` array in `devcontainer.json`.

### Install Additional Tools
Add commands to `Dockerfile` or use `postCreateCommand` in `devcontainer.json`.

## References

- [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)
- [VS Code Dev Containers](https://code.visualstudio.com/docs/devcontainers/containers)
- [Official Tauri DevContainer](https://github.com/tauri-apps/tauri/tree/dev/.devcontainer)
