use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;
use chrono::Local;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

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
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanResult {
    pub file_path: String,
    pub total_lines: u64,
    pub total_size_bytes: u64,
    pub duration_secs: u64,
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

// ── Commands ──────────────────────────────────────────────────────────────────

/// Check if the `tree` command is installed.
#[tauri::command]
fn check_tree() -> bool {
    Command::new("which")
        .arg("tree")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Return OS-specific list of drives / mount points.
#[tauri::command]
fn get_drives() -> Vec<Drive> {
    let mut drives = vec![Drive {
        path: "/".to_string(),
        label: root_label(),
    }];
    additional_drives(&mut drives);
    drives
}

/// Start the `tree` scan on `target_path`, emit `scan-progress` events,
/// and return a `ScanResult` when finished.
#[tauri::command]
async fn start_scan(app: AppHandle, target_path: String) -> Result<ScanResult, String> {
    let output_path = build_output_path(&target_path)?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create snapshot directory: {}", e))?;
    }

    // Clone app handle so it can be moved into spawn_blocking
    let app_clone = app.clone();
    let out = output_path.clone();
    let target = target_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_tree_scan(app_clone, target, out)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

// ── Core scan logic (blocking) ────────────────────────────────────────────────

fn run_tree_scan(
    app: AppHandle,
    target_path: String,
    output_path: PathBuf,
) -> Result<ScanResult, String> {
    let mut child = Command::new("tree")
        .args(["-apu", "-h", &target_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to run tree: {}. Is it installed?", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let reader = BufReader::new(stdout);

    let mut file = std::fs::File::create(&output_path)
        .map_err(|e| format!("Failed to create output file: {}", e))?;

    let start = Instant::now();
    let mut lines: u64 = 0;
    let mut size_bytes: u64 = 0;
    let mut last_emit = Instant::now();

    for line_result in reader.lines() {
        let line = line_result.map_err(|e| format!("Read error: {}", e))?;
        let bytes = line.as_bytes();

        file.write_all(bytes).map_err(|e| format!("Write error: {}", e))?;
        file.write_all(b"\n").map_err(|e| format!("Write error: {}", e))?;

        lines += 1;
        size_bytes += bytes.len() as u64 + 1;

        // Emit progress at most every 100ms to avoid flooding the UI
        if last_emit.elapsed().as_millis() >= 100 {
            let _ = app.emit("scan-progress", ScanProgress {
                lines,
                size_bytes,
                elapsed_secs: start.elapsed().as_secs(),
            });
            last_emit = Instant::now();
        }
    }

    child.wait().map_err(|e| format!("Process wait error: {}", e))?;

    let duration_secs = start.elapsed().as_secs();

    // Emit final state
    let _ = app.emit("scan-progress", ScanProgress {
        lines,
        size_bytes,
        elapsed_secs: duration_secs,
    });

    Ok(ScanResult {
        file_path: output_path.to_string_lossy().to_string(),
        total_lines: lines,
        total_size_bytes: size_bytes,
        duration_secs,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_output_path(target_path: &str) -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable not set".to_string())?;

    let date_str = Local::now().format("%Y%m%d").to_string();

    let drive_name = if target_path == "/" {
        if cfg!(target_os = "macos") {
            "macOS_Root".to_string()
        } else {
            "Linux_Root".to_string()
        }
    } else if target_path.starts_with("/mnt/") && target_path.len() == 7 {
        // WSL: /mnt/c → Windows_C
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

fn root_label() -> String {
    if cfg!(target_os = "macos") {
        "Root Filesystem (macOS /)".to_string()
    } else {
        "Root Filesystem (/)".to_string()
    }
}

fn additional_drives(drives: &mut Vec<Drive>) {
    #[cfg(target_os = "macos")]
    {
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            let mut volumes: Vec<_> = entries.flatten().collect();
            volumes.sort_by_key(|e| e.file_name());
            for entry in volumes {
                let path = entry.path();
                // Skip the symlink that points to "/" (the root volume)
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
                            && n.chars()
                                .next()
                                .map(|c| c.is_ascii_lowercase())
                                .unwrap_or(false)
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
        } else {
            if let Ok(output) = Command::new("lsblk")
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
}

// ── Open External URL ─────────────────────────────────────────────────────────

#[tauri::command]
fn open_url_external(url: String) {
    open_url(&url);
}

// ── Install tree ──────────────────────────────────────────────────────────────

/// Detect the appropriate install command for this OS/distro.
fn get_install_args() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        if which_exists("brew") {
            return Ok(vec!["brew".into(), "install".into(), "tree".into()]);
        }
        return Err(
            "Homebrew is not installed. Visit https://brew.sh to install it, then run: brew install tree".into(),
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

        // WSL usually has passwordless sudo; native Linux uses pkexec for GUI auth
        let elevator = if is_wsl { "sudo" } else { "pkexec" };
        let mut cmd = vec![elevator.to_string()];
        cmd.extend(args.iter().map(|s| s.to_string()));
        return Ok(cmd);
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    Err("Unsupported operating system".into())
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_install(app: AppHandle, args: Vec<String>) -> Result<(), String> {
    let mut child = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start installer: {}", e))?;

    // Stream stderr on a separate thread so it doesn't block stdout reading
    let app_err = app.clone();
    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                let _ = app_err.emit("install-output", &line);
            }
        });
    }

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            let _ = app.emit("install-output", &line);
        }
    }

    let status = child.wait().map_err(|e| format!("Wait error: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err("Installation failed. Check your internet connection or install manually.".into())
    }
}

#[tauri::command]
async fn install_tree(app: AppHandle) -> Result<(), String> {
    let args = get_install_args()?;
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || run_install(app_clone, args))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

// ── Delete Snapshot Log ───────────────────────────────────────────────────────

#[tauri::command]
fn delete_snapshot_log(file_path: String) -> Result<(), String> {
    std::fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to delete file: {}", e))
}

// ── Open File ─────────────────────────────────────────────────────────────────

/// Open a file with the OS default application.
/// Uses `open` on macOS and `xdg-open` on Linux, bypassing the plugin sandbox.
#[tauri::command]
fn open_file(file_path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&file_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    Ok(())
}

// ── Snapshot Logs ─────────────────────────────────────────────────────────────

#[tauri::command]
fn get_snapshot_logs() -> Vec<SnapshotLog> {
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

    // Most recent first (YYYYMMDD prefix sorts correctly)
    logs.sort_by(|a, b| b.filename.cmp(&a.filename));
    logs
}

// ── System Info ───────────────────────────────────────────────────────────────

#[tauri::command]
fn get_system_info() -> SystemInfo {
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
    if version.is_empty() {
        "macOS".to_string()
    } else {
        format!("macOS {}", version)
    }
}

#[cfg(target_os = "linux")]
fn get_os_label() -> String {
    let is_wsl = std::fs::read_to_string("/proc/version")
        .map(|v| v.contains("Microsoft") || v.contains("WSL"))
        .unwrap_or(false);
    let distro = get_linux_distro();
    if is_wsl {
        format!("WSL ({})", distro)
    } else {
        distro
    }
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
                l.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string()
            })
        })
        .unwrap_or_else(|| "Linux".to_string())
}

// ── Entry Point ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            build_menu(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_tree,
            get_drives,
            start_scan,
            get_system_info,
            get_snapshot_logs,
            delete_snapshot_log,
            open_file,
            open_url_external,
            install_tree,
        ])
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "app_github"        => open_url("https://github.com/leejongyoung/TreeSnapshots"),
                "app_releases"      => open_url("https://github.com/leejongyoung/TreeSnapshots/releases"),
                "app_updates"       => open_url("https://github.com/leejongyoung/TreeSnapshots/releases/latest"),
                "app_install_tools" => { let _ = app.emit("open-install-dialog", ()); }
                "app_licenses"      => { let _ = app.emit("open-licenses", ()); }
                "help_issues"       => open_url("https://github.com/leejongyoung/TreeSnapshots/issues"),
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_menu(handle: &tauri::AppHandle) -> tauri::Result<()> {
    let about_meta = AboutMetadata {
        name: Some("TreeSnapshots".into()),
        version: Some("2.0.0".into()),
        authors: Some(vec!["leejongyoung".into()]),
        comments: Some("File System Snapshot Tool\nFile system tree capture utility.".into()),
        website: Some("https://github.com/leejongyoung/TreeSnapshots".into()),
        website_label: Some("GitHub Repository".into()),
        license: Some("MIT".into()),
        ..Default::default()
    };

    // App menu — on macOS this becomes the leftmost menu named after the app
    let app_menu = SubmenuBuilder::new(handle, "TREESNAPSHOTS")
        .item(&PredefinedMenuItem::about(handle, Some("About TreeSnapshots"), Some(about_meta))?)
        .separator()
        .item(&MenuItemBuilder::with_id("app_github", "GitHub Repository").build(handle)?)
        .item(&MenuItemBuilder::with_id("app_releases", "Release Changelog").build(handle)?)
        .item(&MenuItemBuilder::with_id("app_updates", "Check for Updates...").build(handle)?)
        .separator()
        .item(&MenuItemBuilder::with_id("app_install_tools", "Install Dependency Tools").build(handle)?)
        .item(&MenuItemBuilder::with_id("app_licenses", "Open Source Licenses").build(handle)?)
        .separator()
        .item(&PredefinedMenuItem::quit(handle, Some("Quit TreeSnapshots"))?)
        .build()?;

    let edit_menu = SubmenuBuilder::new(handle, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .separator()
        .select_all()
        .build()?;

    let help_menu = SubmenuBuilder::new(handle, "Help")
        .item(&MenuItemBuilder::with_id("help_issues", "Report an Issue").build(handle)?)
        .build()?;

    let menu = MenuBuilder::new(handle)
        .item(&app_menu)
        .item(&edit_menu)
        .item(&help_menu)
        .build()?;

    handle.set_menu(menu)?;
    Ok(())
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    { let _ = Command::new("open").arg(url).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = Command::new("xdg-open").arg(url).spawn(); }
}
