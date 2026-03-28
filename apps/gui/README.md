<div align="center">

# TreeSnapshots GUI 🌳

**A native desktop app for capturing and managing file system tree snapshots.**

![macOS](https://img.shields.io/badge/macOS-Tahoe%2026+-blue.svg?style=for-the-badge&logo=apple)
![Ubuntu](https://img.shields.io/badge/Ubuntu-24.04+-E95420?style=for-the-badge&logo=ubuntu&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-2.x-FFC131?style=for-the-badge&logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=black)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

</div>

---

`treesnap-gui` is a lightweight desktop application built with **Tauri 2**, **React 19**, and **Tailwind CSS v4**. It provides a graphical interface for the TreeSnapshots tool — letting you select a drive, capture its full file system tree, browse past snapshots, and manage them, all without touching a terminal.

---

## 🚀 Quick Start (Development)

### Prerequisites

| Tool | Version | Install |
| :--- | :------ | :------ |
| **Rust** | 1.85 or newer | [rustup.rs](https://rustup.rs) |
| **Node.js** | 18 or newer | [nodejs.org](https://nodejs.org) |
| **Tauri CLI** | 2.x | `cargo install tauri-cli` |
| **tree** | any | `brew install tree` (macOS) |

### Run in Development Mode

```bash
cd treesnap-gui
npm install
npm run tauri dev
```

The app will launch with hot-reload enabled for the frontend.

---

## 📦 Production Build

```bash
cd treesnap-gui
npm run tauri build
```

Output bundles are written to `src-tauri/target/release/bundle/`:

| Platform | Format |
| :------- | :----- |
| macOS    | `.dmg`, `.app` |
| Linux    | `.deb`, `.rpm` |
| Windows  | `.exe` (NSIS) |

> **macOS Note:** Without a valid Apple Developer certificate, the `.app` bundle will be blocked by Gatekeeper. For personal use, right-click → Open to bypass. For public distribution, code signing and notarization are required.

---

## ✨ Core Features

- **🖥️ Drive Selection** — Automatically lists all available drives and mount points, including external volumes on macOS and Windows drives (`C:`, `D:`) under WSL.
- **📸 One-click Snapshots** — Captures the full file system tree with permissions and sizes using `tree -apu -h`, and saves it as a timestamped `.txt` file.
- **⏱️ Real-time Progress** — Streams live line count, file size, and elapsed time while scanning.
- **📋 Snapshot Log Viewer** — Browse, open, and selectively delete past snapshots from within the app.
- **🔧 Dependency Installer** — Detects if `tree` is not installed and offers a guided one-click install via `brew` (macOS) or `apt-get` / `dnf` / `yum` (Linux).
- **🌗 Light / Dark Mode** — Fully adapts to the system appearance preference automatically.
- **🖱️ Native Menu Bar** — Full macOS menu bar integration with About, GitHub, Release Changelog, Check for Updates, and more.
- **🪟 Frameless Window** — Overlay titlebar that blends with the app background, keeping the traffic light buttons native.

---

## 📋 Supported Operating Systems

| Operating System    | Family | Recommended Version | Status      |
| :------------------ | :----- | :------------------ | :---------- |
| **macOS**           | Darwin | Tahoe 26 or newer   | ✅ Verified |
| **Ubuntu**          | Debian | 24.04 LTS or newer  | ✅ Verified |
| **Windows 10 / 11** | WSL    | Latest              | ✅ Verified |

> Windows support is provided via WSL. The app detects the WSL environment automatically and lists Windows drives alongside Linux mount points.

---

## 🛠 How It Works

### 1. Launch the App

On first launch, the app checks whether the `tree` command is available on your system. The status is shown as an **ONLINE** / **OFFLINE** indicator on the main screen.

- **ONLINE** — `tree` is installed and ready to use.
- **OFFLINE** — `tree` is missing. Click the indicator or use the menu bar to trigger the installer.

### 2. Select a Drive

Choose from the automatically detected list of drives:

- **macOS** — Root filesystem (`/`) and any mounted volumes under `/Volumes`
- **Linux** — Root filesystem and any non-system mount points
- **WSL** — Root filesystem and Windows drives (`/mnt/c`, `/mnt/d`, ...)

### 3. Create a Snapshot

Click **Create Snapshot**. The app runs `tree -apu -h` on the selected path and streams output in real time. When the scan completes, the result is saved to:

```
~/TreeSnapshots/snapshots/snapshot_YYYYMMDD_<DriveName>.txt
```

### 4. Manage Snapshots

Switch to the **Snapshot Logs** screen to view all saved snapshots sorted by date. You can open any snapshot in your default text editor or delete individual files directly from the list.

---

## 🏗 Tech Stack

| Layer | Technology |
| :---- | :--------- |
| Shell | Tauri 2 (Rust) |
| UI Framework | React 19 |
| Styling | Tailwind CSS v4 |
| Bundler | Vite 7 |
| Language | TypeScript 5.8 |

---

<div align="center">
  <em>Happy snapshotting!</em>
</div>
