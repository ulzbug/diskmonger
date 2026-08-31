# DiskMonger

DiskMonger is a modern, blazing-fast, and cross-platform disk space visualizer. It is a complete, modern rewrite inspired by the classic Windows utility **SpaceMonger v1.4.0** by Sean B. Palmer, designed to let you understand your disk space usage **at a single glance**.

The project comes in two flavors:
1.  **DiskMonger Desktop**: A modern graphical app built with Tauri v2, HTML5 Canvas, and Rust.
2.  **DiskMonger CLI**: A lightweight, interactive terminal utility (`diskmonger-cli`) built with Rust and Ratatui.

The original C++ MFC source code for SpaceMonger is available on GitHub: [**seanofw/spacemonger1**](https://github.com/seanofw/spacemonger1).

---

## Features (At a Glance)

*   **At-a-Glance Visualization:** See exactly which folders and files are eating up your space using a beautifully balanced, squarified treemap layout.
*   **Real-time Progress:** Stay informed of the progress with a live display of currently scanned files.
*   **Accurate Size Metrics:** Shows the **real space allocated on disk** (based on filesystem clusters), rather than just the logical file sizes.
*   **Intelligent Grouping:** Automatically merges tiny files into a single `[Other Files]` block to keep your treemap clean, readable, and focused on large space-wasters.
*   **Interactive Navigation & Actions:**
    *   **Zoom in** on a directory to explore its sub-folders recursively, and **Zoom out** or **Reset** to return instantly.
    *   **Perform direct actions** right from the view: Open folders, copy absolute paths, reveal in your system's explorer, or send files/directories straight to the Trash.
	*   **Reload** only a subdirectory

---

## DiskMonger CLI (`diskmonger-cli`)

For terminal lovers, system administrators, or SSH sessions, `diskmonger-cli` packs the full power of DiskMonger directly into your terminal!

### Features
*   **Ultra-lightweight & Portable:** A single, self-contained executable binary. No installer or dependencies required—just copy it and run.
*   **Mnemonic-driven Navigation:** Keyboard shortcuts adapt dynamically to your language. Indicators like **<u>Z</u>oomer** or **<u>Q</u>uit** have their shortcut letters underlined right on screen.
*   **Interactive Arrow Controls:**
    *   `Tab` / `◄` / `►`: Move the focus between sibling elements at the exact same depth.
    *   `▼`: Drill down into the first visible child of a folder.
    *   `▲`: Go up to the parent directory (with automatic zoom out).
    *   `Z` / `Enter`: Zoom into the selected directory.
    *   `D`: Zoom out of the current view.
    *   `R`: Reset zoom to the topmost root.
    *   `L`: Toggle partition free-space visibility.
*   **Instant Cancellation:** Press `Esc` at any moment to immediately abort a scan and exit cleanly.

---

## Supported Languages (24)

DiskMonger is localized in 24 major European languages:
*German (de), English (en), Bulgarian (bg), Croatian (hr), Danish (da), Spanish (es), Estonian (et), Finnish (fi), French (fr), Greek (el), Hungarian (hu), Irish (ga), Italian (it), Latvian (lv), Lithuanian (lt), Maltese (mt), Dutch (nl), Polish (pl), Portuguese (pt), Romanian (ro), Slovak (sk), Slovenian (sl), Swedish (sv), and Czech (cs).*

---

## Compilation

First, navigate to the project directory:
```bash
cd diskmonger/
```

### 1. Launching DiskMonger Desktop (Tauri)

#### Prerequisites
Ensure you have the Tauri prerequisites installed on your system (Node.js, Rust, and standard compilation tools).

*   **Linux (Debian/Ubuntu)**:
    ```bash
    sudo apt update && sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
    ```
*   **Windows**: Install **Microsoft C++ Build Tools**, **Rust**, and **Node.js** from their official sites.

#### Install and Run
Install Javascript dependencies:
```bash
npm install
```

To run in development mode:
```bash
npm run tauri dev
```

To compile and pack into a production installer (such as a `.deb` package on Linux or `.msi` on Windows):
```bash
# On Linux, make sure to add cargo to your PATH if needed:
PATH=$PATH:$HOME/.cargo/bin npm run tauri build
```
*The packaged bundles will be generated under `diskmonger/target/release/bundle/`.*

---

### 2. Launching and Compiling DiskMonger CLI

`diskmonger-cli` is compiled purely in Rust.

#### Run in Development Mode
To scan a path instantly:
```bash
cargo run --manifest-path diskmonger-cli/Cargo.toml -- -p /path/to/scan
```

#### Compile Standalone Binary (Linux)
To build a highly optimized standalone binary for your system:
```bash
cargo build --release --manifest-path diskmonger-cli/Cargo.toml
```
*The standalone binary is generated at `target/release/diskmonger-cli`.*


## License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.
