# DiskMonger 🚀

DiskMonger is a modern, fast, and cross-platform disk space visualizer, heavily inspired by the classic Windows utility **SpaceMonger v1.4.0** by Sean B. Palmer.

This project is a complete rewrite from the ground up, built with a modern and performant stack:
*   **[Tauri v2](https://tauri.app/)** for the application framework.
*   **[Rust](https://www.rust-lang.org/)** for the powerful and safe backend.
*   **[TypeScript](https://www.typescriptlang.org/)** and **HTML5 Canvas** for the fluid and responsive user interface.

The original C++ MFC source code for SpaceMonger is available on GitHub in the official repository : [**seanofw/spacemonger1**](https://github.com/seanofw/spacemonger1).

---

## Features

*   **High-Performance Treemap:** Visualizes disk space using a "Squarified Treemap" algorithm for a balanced and readable layout, rendered on a hardware-accelerated HTML5 Canvas.
*   **Asynchronous Scanning:** The UI remains 100% responsive during scans, even on massive drives, thanks to a dedicated Rust background thread.
*   **Real-time Progress:** The window title and a message on the canvas keep you informed of the folder currently being scanned.
*   **Accurate Size Calculation:** Reports the **true size on disk** (allocated space based on filesystem clusters), not just the logical file size.
*   **Intelligent Grouping:** Small files within a directory are automatically grouped into a `[Autres fichiers]` block to keep the display clean and readable.
*   **Interactive Navigation:**
    *   **Double-click** to zoom into a directory.
    *   **Single-click** to select an item and display a detailed tooltip with its name, full path, size, and type.
    *   "Zoom Out" button to logically navigate to the parent directory.
    *   "Reset" button to instantly return to the root of the scan.
*   **Modern UI:** A clean, compact, and dark-themed UI that gets out of your way.

---

## Platform Compatibility

DiskMonger is built with Tauri v2 and is fully cross-platform. It can be compiled and run on:
*   **Windows** 10, 11
*   **macOS**
*   **Linux** (tested on Ubuntu/Debian-based distributions)

---

## Getting Started

### Prerequisites

You must have the Tauri v2 prerequisites installed for your specific operating system.

#### Linux (Debian/Ubuntu/Mint)
Install compilation tools, SSL, and WebKitGTK headers:
```bash
sudo apt update && sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

#### Windows
1.  Install the **Microsoft C++ Build Tools** from the [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-studio-build-tools/).
2.  Install **Rust** via `rustup-init.exe` from the official [Rust website](https://www.rust-lang.org/tools/install).
3.  Install **Node.js** from the official [Node.js website](https://nodejs.org/).

#### Install Rust (on Linux/macOS)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Installation & Launching

First, navigate to the new application's directory:
```bash
cd diskmonger/
```

Then, install the JavaScript dependencies:
```bash
npm install
```

To run the application in **Development Mode** (with hot-reload):
```bash
npm run tauri dev
```

---

## Compiling for Release

To build the final, optimized, and distributable application:

```bash
cd diskmonger/
npm run tauri build
```

This command will produce a small, native installer/executable for your platform in the `diskmonger/src-tauri/target/release/bundle/` directory.

### Creating a Release on GitHub

1.  Commit your final code to Git.
2.  Create a tag for your release version (e.g., `git tag v1.0.0`).
3.  Push the tag to GitHub (`git push origin v1.0.0`).
4.  On the GitHub repository page, go to "Releases" and click "Draft a new release".
5.  Select the tag you just pushed.
6.  **Upload the binaries** (`.exe`, `.msi`, `.deb`, `.AppImage`) from the `bundle` directory as release assets.
7.  Publish the release.

---

## License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.
