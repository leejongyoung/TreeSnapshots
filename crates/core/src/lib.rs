use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use chrono::Local;
use serde::Serialize;

// ── Data Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct Drive {
    pub path: String,
    pub label: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanProgress {
    pub lines: u64,
    pub size_bytes: u64,
    pub elapsed_secs: u64,
    pub recent_entry: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanResult {
    pub file_path: String,
    pub total_lines: u64,
    pub total_size_bytes: u64,
    pub duration_secs: u64,
    pub skipped_dirs: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SystemInfo {
    pub os_label: String,
    pub hostname: String,
    pub username: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SnapshotLog {
    pub filename: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

// ── Utilities ─────────────────────────────────────────────────────────────────

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.2} GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

pub fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── tree check ────────────────────────────────────────────────────────────────

pub fn check_tree() -> bool {
    which_exists("tree")
}

// ── Drives ────────────────────────────────────────────────────────────────────

pub fn get_drives() -> Vec<Drive> {
    let mut drives = vec![Drive {
        path: "/".to_string(),
        label: root_label(),
    }];
    additional_drives(&mut drives);
    drives
}

fn root_label() -> String {
    if cfg!(target_os = "macos") {
        "Root Filesystem (macOS /)".to_string()
    } else {
        "Root Filesystem (/)".to_string()
    }
}

#[allow(unused_variables)]
fn additional_drives(drives: &mut Vec<Drive>) {
    #[cfg(target_os = "macos")]
    {
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            let mut volumes: Vec<_> = entries.flatten().collect();
            volumes.sort_by_key(|e| e.file_name());
            for entry in volumes {
                let path = entry.path();
                if path.is_symlink() {
                    continue;
                }
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                drives.push(Drive {
                    path: path.to_string_lossy().to_string(),
                    label: format!("External Drive ({})", name),
                });
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let is_wsl = std::fs::read_to_string("/proc/version")
            .map(|v| v.contains("Microsoft") || v.contains("WSL"))
            .unwrap_or(false);

        if is_wsl {
            if let Ok(entries) = std::fs::read_dir("/mnt") {
                let mut letters: Vec<_> = entries
                    .flatten()
                    .filter(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.len() == 1
                            && n.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
                    })
                    .collect();
                letters.sort_by_key(|e| e.file_name());
                for entry in letters {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let letter = name.to_uppercase();
                    drives.push(Drive {
                        path: format!("/mnt/{}", name),
                        label: format!("Windows Drive ({}:)", letter),
                    });
                }
            }
        } else if let Ok(output) = Command::new("lsblk")
            .args(["-o", "MOUNTPOINT", "-n", "-r"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let excluded = ["/", "[SWAP]", ""];
            for line in text.lines() {
                let mp = line.trim();
                if excluded.contains(&mp)
                    || mp.starts_with("/boot")
                    || mp.starts_with("/snap")
                {
                    continue;
                }
                drives.push(Drive {
                    path: mp.to_string(),
                    label: format!("Mount Point ({})", mp),
                });
            }
        }
    }
}

// ── Output Path ───────────────────────────────────────────────────────────────

pub fn build_output_path(target_path: &str) -> Result<PathBuf, String> {
    // When run with sudo, prefer the real user's home over /var/root
    let home = if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        Command::new("sh")
            .args(["-c", &format!("eval echo ~{}", sudo_user)])
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() || s.starts_with('~') { None } else { Some(s) }
            })
            .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
    } else {
        std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?
    };

    let date_str = Local::now().format("%Y%m%d").to_string();

    let drive_name = if target_path == "/" {
        if cfg!(target_os = "macos") {
            "macOS_Root".to_string()
        } else {
            "Linux_Root".to_string()
        }
    } else if target_path.starts_with("/mnt/") && target_path.len() == 7 {
        let letter = target_path
            .chars()
            .last()
            .unwrap_or('x')
            .to_uppercase()
            .next()
            .unwrap_or('X');
        format!("Windows_{}", letter)
    } else {
        std::path::Path::new(target_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .replace(' ', "_")
    };

    Ok(PathBuf::from(format!(
        "{}/TreeSnapshots/snapshots/snapshot_{}_{}.txt",
        home, date_str, drive_name
    )))
}

// ── Scan ─────────────────────────────────────────────────────────────────────

pub fn run_tree_scan(
    target_path: String,
    output_path: PathBuf,
    on_progress: impl Fn(ScanProgress) + Send + 'static,
) -> Result<ScanResult, String> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create snapshot directory: {}", e))?;
    }

    let mut cmd_args: Vec<String> = vec![
        "-apu".into(), "-h".into(), "-x".into(),
    ];

    // Determine the actual scan targets (may differ from target_path for non-root root scans).
    // Extra paths are pushed at the end of cmd_args after all flags.
    let mut scan_targets: Vec<String> = Vec::new();

    // Cloud & network storage — can trigger network sync or block on remote I/O.
    let cloud_exclude =
        // macOS File Provider mount point (Monterey+): ~/Library/CloudStorage
        "CloudStorage|\
         \
         Google Drive|Google Drive (My Drive)|Google Drive (Shared drives)|\
         Dropbox|\
         OneDrive|OneDrive - Personal|\
         Box Sync|Box Drive|\
         iCloud Drive|Mobile Documents|\
         MEGA|\
         pCloud Drive|\
         Tresorit|\
         Sync|\
         Amazon Drive|Amazon Photos|\
         Creative Cloud Files|\
         Nextcloud|ownCloud|\
         Seafile|\
         Insync|\
         \
         SynologyDrive|Synology Drive|\
         Qsync|QNAP Qsync|\
         WD Sync|WD Drive|\
         ShareFile|\
         Egnyte Drive";

    // Package manager & build tool caches — can contain millions of files,
    // are not meaningful in a snapshot, and often cause tree to stall.
    let cache_exclude =
        ".m2|\
         .gradle|\
         .npm|node_modules|\
         .cargo|\
         .cache|\
         .nvm|\
         .pyenv|.venv|__pycache__|\
         .rbenv|.rvm|\
         .local";

    let user_exclude = format!("{}|{}", cloud_exclude, cache_exclude);

    #[cfg(target_os = "macos")]
    if target_path == "/" {
        let running_as_root = Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false);

        if running_as_root {
            // Full root scan with system-internal exclusions
            cmd_args.push("-I".into());
            // .vol/.nofollow/.resolve/.file  — APFS virtual/internal entries
            // System|cores              — SIP-protected OS internals
            // .Spotlight-V100 etc.      — index databases
            // run                       — /private/var/run (Unix domain sockets)
            // folders|vm                — /private/var/folders (app temp), /private/var/vm (swap)
            cmd_args.push(
                ".vol|.nofollow|.resolve|.file|\
                 System|cores|\
                 .Spotlight-V100|.fseventsd|.DocumentRevisions-V100|.MobileBackups|\
                 run|folders|vm"
                .into(),
            );
            scan_targets.push("/".into());
        } else {
            // Non-root: /Applications, /opt, home directory — cloud storage excluded
            let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/Shared".to_string());
            cmd_args.push("-I".into());
            cmd_args.push(user_exclude.clone());
            scan_targets.push("/Applications".into());
            scan_targets.push("/opt".into());
            scan_targets.push(home);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let is_linux_root = target_path == "/";
        // WSL Windows drive: /mnt/c, /mnt/d, … (single letter after /mnt/)
        let is_wsl_drive = target_path.starts_with("/mnt/")
            && target_path.len() == 6
            && target_path.as_bytes()[5].is_ascii_lowercase();

        let running_as_root = Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false);

        if is_linux_root {
            if running_as_root {
                // Full root scan with virtual-fs exclusions
                cmd_args.push("-I".into());
                cmd_args.push("proc|sys|dev|run|tmp|lost+found".into());
                scan_targets.push("/".into());
            } else {
                // Non-root: /home, /opt, /usr/local — cloud storage excluded
                cmd_args.push("-I".into());
                cmd_args.push(user_exclude.clone());
                scan_targets.push("/home".into());
                scan_targets.push("/opt".into());
                scan_targets.push("/usr/local".into());
            }
        } else if is_wsl_drive {
            if running_as_root {
                // Full Windows drive scan with OS-internal exclusions
                cmd_args.push("-I".into());
                cmd_args.push(
                    "Windows|$Recycle.Bin|System Volume Information|Recovery|$WinREAgent"
                    .into(),
                );
                scan_targets.push(target_path.clone());
            } else {
                // Non-root: Users / Program Files — cloud storage excluded
                cmd_args.push("-I".into());
                cmd_args.push(user_exclude.clone());
                scan_targets.push(format!("{}/Users", target_path));
                scan_targets.push(format!("{}/Program Files", target_path));
                scan_targets.push(format!("{}/Program Files (x86)", target_path));
            }
        }
    }

    if scan_targets.is_empty() {
        scan_targets.push(target_path.clone());
    }
    cmd_args.extend(scan_targets);

    let mut child = Command::new("tree")
        .args(&cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to run tree: {}. Is it installed?", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let reader = BufReader::new(stdout);

    let mut file = std::fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let start = Instant::now();

    // Shared counters — written by the scan loop, read by the progress thread
    let shared_lines   = Arc::new(AtomicU64::new(0));
    let shared_size    = Arc::new(AtomicU64::new(0));
    let shared_skipped = Arc::new(AtomicU64::new(0));
    let shared_recent: Arc<RwLock<String>> = Arc::new(RwLock::new(String::new()));
    let done           = Arc::new(AtomicBool::new(false));

    // Progress thread: fires every 200 ms regardless of whether tree is producing output
    {
        let lines  = shared_lines.clone();
        let size   = shared_size.clone();
        let recent = shared_recent.clone();
        let done   = done.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(200));
                if done.load(Ordering::Relaxed) { break; }
                let recent_entry = recent.read()
                    .map(|r| r.clone())
                    .unwrap_or_default();
                on_progress(ScanProgress {
                    lines:        lines.load(Ordering::Relaxed),
                    size_bytes:   size.load(Ordering::Relaxed),
                    elapsed_secs: start.elapsed().as_secs(),
                    recent_entry,
                });
            }
        });
    }

    for line_result in reader.lines() {
        let line = line_result.map_err(|e| format!("Read error: {}", e))?;
        let bytes = line.as_bytes();

        // Drop permission-denied entries — skip writing and counting them
        if line.contains("[error opening dir]") {
            shared_skipped.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        file.write_all(bytes).map_err(|e| format!("Write error: {}", e))?;
        file.write_all(b"\n").map_err(|e| format!("Write error: {}", e))?;

        shared_lines.fetch_add(1, Ordering::Relaxed);
        shared_size.fetch_add(bytes.len() as u64 + 1, Ordering::Relaxed);

        // Extract the file/dir name from tree's formatted output:
        // e.g. "├── [drwxr-xr-x user  group]  dirname" → "dirname"
        let entry_name = line.rfind(']')
            .map(|i| line[i + 1..].trim())
            .filter(|s| !s.is_empty())
            .unwrap_or(line.trim());
        if !entry_name.is_empty() {
            if let Ok(mut w) = shared_recent.write() {
                *w = entry_name.to_string();
            }
        }
    }

    done.store(true, Ordering::Relaxed);
    child.wait().map_err(|e| format!("Process wait error: {}", e))?;

    let total_lines      = shared_lines.load(Ordering::Relaxed);
    let total_size_bytes = shared_size.load(Ordering::Relaxed);
    let duration_secs    = start.elapsed().as_secs();
    let skipped_dirs     = shared_skipped.load(Ordering::Relaxed);

    Ok(ScanResult {
        file_path: output_path.to_string_lossy().to_string(),
        total_lines,
        total_size_bytes,
        duration_secs,
        skipped_dirs,
    })
}

// ── System Info ───────────────────────────────────────────────────────────────

pub fn get_system_info() -> SystemInfo {
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    let hostname = Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    SystemInfo {
        os_label: get_os_label(),
        hostname,
        username,
    }
}

#[cfg(target_os = "macos")]
fn get_os_label() -> String {
    let version = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if version.is_empty() { "macOS".to_string() } else { format!("macOS {}", version) }
}

#[cfg(target_os = "linux")]
fn get_os_label() -> String {
    let is_wsl = std::fs::read_to_string("/proc/version")
        .map(|v| v.contains("Microsoft") || v.contains("WSL"))
        .unwrap_or(false);
    let distro = get_linux_distro();
    if is_wsl { format!("WSL ({})", distro) } else { distro }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_os_label() -> String {
    "Unknown OS".to_string()
}

#[cfg(target_os = "linux")]
fn get_linux_distro() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content.lines().find(|l| l.starts_with("PRETTY_NAME=")).map(|l| {
                l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string()
            })
        })
        .unwrap_or_else(|| "Linux".to_string())
}

// ── Snapshot Logs ─────────────────────────────────────────────────────────────

pub fn get_snapshot_logs() -> Vec<SnapshotLog> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let dir = format!("{}/TreeSnapshots/snapshots", home);

    let mut logs = Vec::new();

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return logs;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let Ok(metadata) = std::fs::metadata(&path) else { continue };

        let size_bytes = metadata.len();
        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                dt.format("%Y-%m-%d %H:%M").to_string()
            })
            .unwrap_or_else(|| "—".to_string());

        logs.push(SnapshotLog {
            filename: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string(),
            file_path: path.to_string_lossy().to_string(),
            size_bytes,
            modified_at,
        });
    }

    logs.sort_by(|a, b| b.filename.cmp(&a.filename));
    logs
}

pub fn delete_snapshot_log(file_path: &str) -> Result<(), String> {
    std::fs::remove_file(file_path)
        .map_err(|e| format!("Failed to delete file: {}", e))
}

// ── Open File ─────────────────────────────────────────────────────────────────

#[allow(unused_variables)]
pub fn open_file(file_path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    Command::new("open")
        .arg(file_path)
        .spawn()
        .map_err(|e| format!("Failed to open file: {}", e))?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open")
        .arg(file_path)
        .spawn()
        .map_err(|e| format!("Failed to open file: {}", e))?;

    Ok(())
}

// ── Install tree ──────────────────────────────────────────────────────────────

pub fn get_install_args() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        if which_exists("brew") {
            return Ok(vec!["brew".into(), "install".into(), "tree".into()]);
        }
        return Err(
            "Homebrew not found. Visit https://brew.sh, then run: brew install tree".into(),
        );
    }

    #[cfg(target_os = "linux")]
    {
        let is_wsl = std::fs::read_to_string("/proc/version")
            .map(|v| v.contains("Microsoft") || v.contains("WSL"))
            .unwrap_or(false);

        let pkg_args: Option<Vec<&str>> = if which_exists("apt-get") {
            Some(vec!["apt-get", "install", "-y", "tree"])
        } else if which_exists("dnf") {
            Some(vec!["dnf", "install", "-y", "tree"])
        } else if which_exists("yum") {
            Some(vec!["yum", "install", "-y", "tree"])
        } else {
            None
        };

        let Some(args) = pkg_args else {
            return Err(
                "No supported package manager found (apt-get, dnf, yum). Install tree manually.".into(),
            );
        };

        let elevator = if is_wsl { "sudo" } else { "pkexec" };
        let mut cmd = vec![elevator.to_string()];
        cmd.extend(args.iter().map(|s| s.to_string()));
        return Ok(cmd);
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    Err("Unsupported operating system".into())
}

pub fn run_install(
    args: Vec<String>,
    on_output: impl Fn(&str) + Send + Sync + 'static,
) -> Result<(), String> {
    let on_output = std::sync::Arc::new(on_output);
    let on_output_stderr = on_output.clone();

    let mut child = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start installer: {}", e))?;

    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                on_output_stderr(&line);
            }
        });
    }

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            on_output(&line);
        }
    }

    let status = child.wait().map_err(|e| format!("Wait error: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err("Installation failed. Check your internet connection or install manually.".into())
    }
}
