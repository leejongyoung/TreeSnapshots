use tauri::{AppHandle, Emitter};
use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use treesnap_core::{Drive, ScanResult, SnapshotLog, SystemInfo};

// ── Tauri Commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn check_tree() -> bool {
    treesnap_core::check_tree()
}

#[tauri::command]
fn get_drives() -> Vec<Drive> {
    treesnap_core::get_drives()
}

#[tauri::command]
async fn start_scan(app: AppHandle, target_path: String) -> Result<ScanResult, String> {
    let output_path = treesnap_core::build_output_path(&target_path)?;
    let app_clone = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        treesnap_core::run_tree_scan(target_path, output_path, move |p| {
            let _ = app_clone.emit("scan-progress", p);
        })
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    treesnap_core::get_system_info()
}

#[tauri::command]
fn get_snapshot_logs() -> Vec<SnapshotLog> {
    treesnap_core::get_snapshot_logs()
}

#[tauri::command]
fn delete_snapshot_log(file_path: String) -> Result<(), String> {
    treesnap_core::delete_snapshot_log(&file_path)
}

#[tauri::command]
fn open_file(file_path: String) -> Result<(), String> {
    treesnap_core::open_file(&file_path)
}

#[tauri::command]
fn open_url_external(url: String) {
    open_url(&url);
}

#[tauri::command]
async fn install_tree(app: AppHandle) -> Result<(), String> {
    let args = treesnap_core::get_install_args()?;
    let app_clone = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        treesnap_core::run_install(args, move |line| {
            let _ = app_clone.emit("install-output", line);
        })
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
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

// ── Menu ──────────────────────────────────────────────────────────────────────

fn build_menu(handle: &tauri::AppHandle) -> tauri::Result<()> {
    let about_meta = AboutMetadata {
        name: Some("TreeSnapshots".into()),
        version: Some("2.0.0".into()),
        authors: Some(vec!["leejongyoung".into()]),
        comments: Some("File system tree capture utility.".into()),
        website: Some("https://github.com/leejongyoung/TreeSnapshots".into()),
        website_label: Some("GitHub Repository".into()),
        license: Some("MIT".into()),
        ..Default::default()
    };

    let app_menu = SubmenuBuilder::new(handle, "TREESNAPSHOTS")
        .item(&PredefinedMenuItem::about(handle, Some("About TreeSnapshots"), Some(about_meta))?)
        .separator()
        .item(&MenuItemBuilder::with_id("app_github",   "GitHub Repository").build(handle)?)
        .item(&MenuItemBuilder::with_id("app_releases", "Release Changelog").build(handle)?)
        .item(&MenuItemBuilder::with_id("app_updates",  "Check for Updates...").build(handle)?)
        .separator()
        .item(&MenuItemBuilder::with_id("app_install_tools", "Install Dependency Tools").build(handle)?)
        .item(&MenuItemBuilder::with_id("app_licenses",      "Open Source Licenses").build(handle)?)
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

#[allow(unused_variables)]
fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    { let _ = std::process::Command::new("open").arg(url).spawn(); }
    #[cfg(target_os = "linux")]
    { let _ = std::process::Command::new("xdg-open").arg(url).spawn(); }
}
