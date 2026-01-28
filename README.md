<div align="center">

#  TreeSnapshots 🌳

**A script-based tool for creating periodic snapshots of your file system structure.**

![macOS](https://img.shields.io/badge/macOS-Tahoe%2026+-blue.svg?style=for-the-badge&logo=apple)
![Ubuntu](https://img.shields.io/badge/Ubuntu-24.04+-E95420?style=for-the-badge&logo=ubuntu&logoColor=white)
![Rocky Linux](https://img.shields.io/badge/Rocky_Linux-10+-10B981?style=for-the-badge&logo=rocky-linux&logoColor=white)
![Fedora](https://img.shields.io/badge/Fedora-43+-51A2DA?style=for-the-badge&logo=fedora&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011%20(WSL)-0078D6?style=for-the-badge&logo=windows&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

</div>

---

This repository contains `TreeSnapshots`, a powerful script that automates the process of recording file system structures on **macOS**, **Linux**, and **Windows (via WSL)**. It uses Git to efficiently track changes over time.

## 🚀 Quick Start

To take a snapshot of your file system, clone this repository and use the `make` command.

> **Note:** This command requires `sudo` to scan system directories and will prompt for your password.

```bash
git clone https://github.com/leejongyoung/TreeSnapshots.git
cd TreeSnapshots
sudo make snapshot
```

---

## ✨ Core Features

This project is managed by a `Makefile` and powered by the `treesnap.sh` script. Key features include:

-   **✅ Simple Interface**: A clean `make snapshot` command is all you need to get started.
-   **🤖 Cross-Platform**: Works seamlessly on macOS, native Linux, and Windows via WSL.
-   **💾 Interactive Drive Selection**: Automatically lists system and external drives, including Windows drives (e.g., C:) under WSL.
-   **⏱️ Real-time Monitoring**: Displays live feedback on the number of lines processed and the size of the snapshot file.
-   **🔐 Automated Permissions**: Runs with `sudo` for full access, then restores ownership of the snapshot file to the original user.
-   **📈 Git-Friendly Format**: Saves snapshots as plain text, making it easy to track changes (`diff`) and keeping the repository lightweight.

---

## 📋 Supported Operating Systems

The script is tested and maintained for the following operating systems and versions.

| Operating System            | Family        | Recommended Version      | Status          |
| :-------------------------- | :------------ | :----------------------- | :-------------- |
| **macOS**                   | `Darwin`      | Tahoe 26 or newer        | ✅ Verified     |
| **Ubuntu**                  | Debian-based  | 24.04 LTS or newer       | ✅ Verified     |
| **Rocky Linux**             | RHEL-based    | Version 10 or newer      | ✅ Verified     |
| **Fedora**                  | RHEL-based    | Version 43 or newer      | ✅ Verified     |
| **Windows 10 / 11**         | `NT (WSL)`    | Latest                   | ✅ Verified     |

> **Note**: On Linux or WSL, the `tree` command is required. If not installed, the script will attempt to install it for you.

---

## 💻 Windows Support (via WSL)

This script runs on Windows thanks to the **Windows Subsystem for Linux (WSL)**. WSL allows you to run a genuine Linux environment directly on Windows, without the overhead of a traditional virtual machine.

### How to Use on Windows

1.  **Install WSL**: If you don't have it, install WSL with a Linux distribution like Ubuntu. Follow the [Official Microsoft Guide](https://learn.microsoft.com/en-us/windows/wsl/install).
2.  **Open Your WSL Terminal**: Launch your Linux distribution (e.g., Ubuntu) from the Start Menu.
3.  **Follow the Quick Start**: Inside the WSL terminal, follow the standard [Quick Start](#-quick-start) instructions. The script will automatically detect the WSL environment and show your Windows drives (C:, D:, etc.) as scanning options.

---

## 🛠 How It Works

Follow these steps to generate and save a snapshot.

### 1. Run the Snapshot Command
This will prompt you to select a drive to scan.

```bash
sudo make snapshot
```

### 2. Commit the Changes
Once the script is finished, a new text file will be created in the `snapshots/` directory. Use Git to commit this file to your repository's history.

```bash
git add snapshots/*.txt
git commit -m "feat: Add new snapshot for $(date +'%Y-%m-%d')"
git push origin main
```

### 3. Review the History
You can now check the `git log` or use `git diff` on the snapshot files to see what has changed between different points in time.

---

<div align="center">
  <em>Happy snapshotting!</em>
</div>
