use std::process::Command;
use std::time::Duration;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, Select};

use treesnap_core::{
    build_output_path, check_tree, delete_snapshot_log, format_bytes, get_drives, get_install_args,
    get_snapshot_logs, run_install, run_tree_scan, which_exists,
};

fn main() {
    // ── Version ───────────────────────────────────────────────────────────────
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("treesnap {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    println!();

    // ── Dependency check ──────────────────────────────────────────────────────
    if !check_tree() {
        eprintln!(
            "  {} 'tree' command not found.",
            style("⚠").yellow().bold()
        );
        println!();

        let install = Confirm::new("Install it now?")
            .with_default(false)
            .prompt()
            .unwrap_or(false);

        if install {
            let sp = spinner("Installing tree...");
            let args = match get_install_args() {
                Ok(a) => a,
                Err(e) => {
                    sp.finish_and_clear();
                    exit_err(&e);
                }
            };
            let sp_clone = sp.clone();
            match run_install(args, move |line| {
                sp_clone.println(format!("  {}", style(line).dim()));
            }) {
                Ok(_) => {
                    sp.finish_and_clear();
                    println!("  {} 'tree' installed successfully.\n", style("✓").green().bold());
                }
                Err(e) => {
                    sp.finish_and_clear();
                    exit_err(&e);
                }
            }
        } else {
            println!();
            println!("  Install manually:");
            println!("    macOS   {}",  style("brew install tree").cyan());
            println!("    Ubuntu  {}", style("sudo apt-get install tree").cyan());
            println!();
            std::process::exit(1);
        }
    }

    // ── Snapshot list + drive selection (loop for back-navigation) ───────────
    const NEW_SNAPSHOT: &str = "+ New Snapshot";
    const QUIT: &str = "q  Quit";

    let selected;
    'menu: loop {
        // ── Existing snapshots ────────────────────────────────────────────────
        let logs = get_snapshot_logs();
        let has_logs = !logs.is_empty();

        if has_logs {
            let log_labels: Vec<String> = logs
                .iter()
                .map(|l| {
                    format!(
                        "{}  {}  {}",
                        l.filename,
                        format_bytes(l.size_bytes),
                        l.modified_at
                    )
                })
                .collect();

            let mut options = vec![NEW_SNAPSHOT.to_string()];
            options.extend(log_labels.iter().cloned());
            options.push(QUIT.to_string());

            match Select::new("Snapshots", options).prompt() {
                Ok(choice) if choice == NEW_SNAPSHOT => {
                    println!();
                }
                Ok(choice) if choice == QUIT => std::process::exit(0),
                Ok(choice) => {
                    let pos = log_labels.iter().position(|l| *l == choice).unwrap();
                    let log = &logs[pos];

                    let action = Select::new(
                        &log.filename,
                        vec!["Open", "Delete", "← Back"],
                    )
                    .prompt();

                    match action {
                        Ok("Open") => {
                            if let Err(e) = open_in_editor(&log.file_path) {
                                eprintln!("  {} {}", style("✗").red().bold(), e);
                            }
                            println!();
                            std::process::exit(0);
                        }
                        Ok("Delete") => {
                            match delete_snapshot_log(&log.file_path) {
                                Ok(_) => println!(
                                    "  {} Deleted {}\n",
                                    style("✓").green().bold(),
                                    style(&log.filename).dim()
                                ),
                                Err(e) => eprintln!(
                                    "  {} {}\n",
                                    style("✗").red().bold(),
                                    e
                                ),
                            }
                            continue 'menu;
                        }
                        _ => continue 'menu,
                    }
                }
                Err(_) => std::process::exit(0),
            }
        }

        // ── Drive selection ───────────────────────────────────────────────────
        let drives = get_drives();
        let mut labels: Vec<String> = drives.iter().map(|d| d.label.clone()).collect();
        labels.push(QUIT.to_string());

        match Select::new("Select a drive to snapshot", labels).prompt() {
            Ok(label) if label == QUIT => std::process::exit(0),
            Ok(label) => {
                selected = drives.into_iter().find(|d| d.label == label).unwrap();
                break 'menu;
            }
            Err(_) => {
                if has_logs {
                    println!();
                    continue 'menu;
                } else {
                    std::process::exit(0);
                }
            }
        }
    }

    // ── Output path ───────────────────────────────────────────────────────────
    let output_path = match build_output_path(&selected.path) {
        Ok(p) => p,
        Err(e) => exit_err(&e),
    };

    // ── Permission warning ────────────────────────────────────────────────────
    if !is_root() {
        let is_wsl_drive = selected.path.starts_with("/mnt/")
            && selected.path.len() == 6
            && selected.path.as_bytes()[5].is_ascii_lowercase();

        let (paths_hint, is_limited) = if selected.path == "/" {
            #[cfg(target_os = "macos")]
            let hint = "/Applications  /opt  ~/";
            #[cfg(target_os = "linux")]
            let hint = "/home  /opt  /usr/local";
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let hint = "";
            (hint, true)
        } else if is_wsl_drive {
            ("Users  Program Files  Program Files (x86)", true)
        } else {
            ("", false)
        };

        if is_limited {
            println!();
            println!(
                "  {} Scanning user-accessible paths only:",
                style("ℹ").cyan().bold()
            );
            println!("    {}", style(paths_hint).dim());
            println!(
                "    Run {} for a full system scan.",
                style("sudo treesnap").cyan()
            );
        }
    }

    // ── Scan ──────────────────────────────────────────────────────────────────
    println!();
    let pb = spinner(format!("Scanning {}...", style(&selected.path).dim()).as_str());
    let pb_clone = pb.clone();

    let result = run_tree_scan(selected.path.clone(), output_path, move |p| {
        let entry = if p.recent_entry.len() > 60 {
            format!("…{}", &p.recent_entry[p.recent_entry.len() - 59..])
        } else {
            p.recent_entry.clone()
        };
        pb_clone.set_message(format!(
            "Scanning  {} lines · {} · {}s\n  {} {}",
            style(p.lines).cyan().bold(),
            style(format_bytes(p.size_bytes)).cyan().bold(),
            p.elapsed_secs,
            style("└").dim(),
            style(entry).dim(),
        ));
    });

    pb.finish_and_clear();
    println!();

    // ── Result ────────────────────────────────────────────────────────────────
    match result {
        Ok(r) => {
            println!("  {} Snapshot saved", style("✓").green().bold());
            println!();
            println!("  {}  {}", style("Lines").dim(),   style(r.total_lines).bold());
            println!("  {}   {}", style("Size").dim(),    style(format_bytes(r.total_size_bytes)).bold());
            println!("  {}   {}", style("Time").dim(),    style(format!("{}s", r.duration_secs)).bold());
            println!("  {}   {}", style("Path").dim(),    style(&r.file_path).underlined());

            if r.skipped_dirs > 0 {
                println!(
                    "  {} {}",
                    style("Skipped").dim(),
                    style(format!("{} directories (permission denied)", r.skipped_dirs)).yellow()
                );
                if !is_root() {
                    println!();
                    println!(
                        "  {} Run {} for a complete scan.",
                        style("💡").dim(),
                        style("sudo treesnap").cyan()
                    );
                }
            }

            println!();
            if Confirm::new("Open snapshot file?")
                .with_default(false)
                .prompt()
                .unwrap_or(false)
            {
                if let Err(e) = open_in_editor(&r.file_path) {
                    eprintln!("  {} {}", style("✗").red().bold(), e);
                }
            }
        }
        Err(e) => {
            eprintln!("  {} {}", style("✗").red().bold(), e);
            std::process::exit(1);
        }
    }

    println!();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(msg.to_string());
    pb
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

fn open_in_editor(file_path: &str) -> Result<(), String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            for candidate in &["nano", "vim", "vi"] {
                if which_exists(candidate) {
                    return candidate.to_string();
                }
            }
            "vi".to_string()
        });

    Command::new(&editor)
        .arg(file_path)
        .status()
        .map(|_| ())
        .map_err(|e| format!("Failed to open editor '{}': {}", editor, e))
}

fn exit_err(msg: &str) -> ! {
    eprintln!("  {} {}", style("✗").red().bold(), msg);
    std::process::exit(1);
}
