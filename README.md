<div align="center">

# TreeSnapshots 🌳

**A tool for capturing and managing file system tree snapshots — available as a CLI and a native desktop GUI.**

![macOS](https://img.shields.io/badge/macOS-Tahoe%2026+-blue.svg?style=for-the-badge&logo=apple)
![Ubuntu](https://img.shields.io/badge/Ubuntu-24.04+-E95420?style=for-the-badge&logo=ubuntu&logoColor=white)
![Rocky Linux](https://img.shields.io/badge/Rocky_Linux-10+-10B981?style=for-the-badge&logo=rocky-linux&logoColor=white)
![Fedora](https://img.shields.io/badge/Fedora-43+-51A2DA?style=for-the-badge&logo=fedora&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011%20(WSL)-0078D6?style=for-the-badge&logo=windows&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

</div>

---

TreeSnapshots records your file system structure using `tree` and saves it as a plain-text snapshot. Both a **CLI binary** and a **native desktop GUI** are provided, sharing the same core logic built in Rust.

---

## 📦 Download

Download the latest release from [GitHub Releases](https://github.com/leejongyoung/TreeSnapshots/releases/latest).

### Homebrew (macOS / Linux)

```bash
brew install leejongyoung/cask/treesnapshots-cli      # CLI (macOS & Linux)
brew install --cask leejongyoung/cask/treesnapshots    # GUI (macOS only)
```

### CLI (`treesnapshots-cli`)

| Platform | File |
| :------- | :--- |
| macOS (Universal) | `treesnapshots-cli-macos-universal.tar.gz` |
| Linux x86_64      | `treesnapshots-cli-linux-x86_64.tar.gz` |
| Windows x86_64    | `treesnapshots-cli-windows-x86_64.zip` |

### Desktop GUI

| Platform | File |
| :------- | :--- |
| macOS (Universal) | `TREESNAPSHOTS_*_universal.dmg` |
| Linux             | `TREESNAPSHOTS_*.deb` / `*.rpm` |
| Windows           | `TREESNAPSHOTS_*_x64-setup.exe` |

> See [`treesnap-gui/README.md`](treesnap-gui/README.md) for GUI-specific documentation.

---

## 📋 Supported Operating Systems

| Operating System    | Family | Recommended Version | CLI | GUI |
| :------------------ | :----- | :------------------ | :-- | :-- |
| **macOS**           | Darwin | Tahoe 26 or newer   | ✅ Verified   | ✅ Verified   |
| **Ubuntu**          | Debian | 24.04 LTS or newer  | ✅ Verified   | ✅ Verified   |
| **Rocky Linux**     | RHEL   | Version 10 or newer | ⚠️ Compatible | ⚠️ Compatible |
| **Fedora**          | RHEL   | Version 43 or newer | ⚠️ Compatible | ⚠️ Compatible |
| **Windows 10 / 11** | —      | Latest              | ✅ Verified   | ✅ Verified   |

> ⚠️ **Compatible**: Built on Ubuntu x86_64. Binary and `.rpm` package are expected to work but not explicitly tested on this platform.

---

## 🔧 Development

### Prerequisites

| Tool | Purpose | Install |
| :--- | :------ | :------ |
| **Rust** | CLI & GUI backend | [rustup.rs](https://rustup.rs) |
| **Node.js** | GUI frontend | [nodejs.org](https://nodejs.org) |
| **just** | Command runner | `brew install just` |

### Commands

```bash
just              # show all available recipes

just dev-cli      # run CLI in development mode
just dev-gui      # run GUI in development mode (hot reload)

just build-cli    # build CLI binary  → target/release/treesnapshots-cli
just build-gui    # build GUI app     → apps/gui/src-tauri/target/release/bundle/

just install      # install treesnapshots-cli globally (~/.cargo/bin)
just uninstall    # uninstall treesnapshots-cli from ~/.cargo/bin
just clean        # clean build artifacts
```

---

## 🏗 Project Structure

```
TreeSnapshots/
├── Justfile               # command runner
├── crates/
│   └── core/              # shared Rust library (scan, drives, logs, install)
└── apps/
    ├── cli/               # treesnapshots binary
    └── gui/               # Tauri 2 + React desktop GUI
```

The CLI and GUI share the same `treesnap-core` library, so behaviour is always consistent between both interfaces.

---

<div align="center">
  <em>Happy snapshotting!</em>
</div>
