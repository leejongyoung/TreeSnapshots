import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "./App.css";

// ── Types ─────────────────────────────────────────────────────────────────────

interface SystemInfo {
  os_label: string;
  hostname: string;
  username: string;
}

interface Drive {
  path: string;
  label: string;
}

interface ScanProgress {
  lines: number;
  size_bytes: number;
  elapsed_secs: number;
  recent_entry: string;
}

interface ScanResult {
  file_path: string;
  total_lines: number;
  total_size_bytes: number;
  duration_secs: number;
}

interface SnapshotLog {
  filename: string;
  file_path: string;
  size_bytes: number;
  modified_at: string;
}

type Screen = "menu" | "select" | "scanning" | "complete" | "logs" | "licenses";

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatDuration(secs: number): string {
  if (secs < 60) return `${secs}s`;
  return `${Math.floor(secs / 60)}m ${secs % 60}s`;
}

function parseLogFilename(filename: string): string {
  const stem = filename.replace(/\.txt$/, "").replace(/^snapshot_/, "");
  const match = stem.match(/^(\d{4})(\d{2})(\d{2})_(.+)$/);
  if (!match) return stem;
  const [, y, m, d, name] = match;
  return `${name.replace(/_/g, " ")} · ${y}-${m}-${d}`;
}

// ── Root Component ────────────────────────────────────────────────────────────

export default function App() {
  const [screen, setScreen] = useState<Screen>("menu");
  const [drives, setDrives] = useState<Drive[]>([]);
  const [treeInstalled, setTreeInstalled] = useState<boolean | null>(null);
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [selectedDrive, setSelectedDrive] = useState<Drive | null>(null);
  const [progress, setProgress] = useState<ScanProgress>({ lines: 0, size_bytes: 0, elapsed_secs: 0, recent_entry: "" });
  const [scanLog, setScanLog] = useState<string[]>([]);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showInstallDialog, setShowInstallDialog] = useState(false);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    Promise.all([
      invoke<Drive[]>("get_drives"),
      invoke<boolean>("check_tree"),
      invoke<SystemInfo>("get_system_info"),
    ]).then(([d, t, s]) => {
      setDrives(d);
      setTreeInstalled(t);
      setSystemInfo(s);
      if (!t) setShowInstallDialog(true);
    });

    const unlistenLicenses = listen("open-licenses", () => setScreen("licenses"));
    const unlistenInstall = listen("open-install-dialog", () => setShowInstallDialog(true));

    return () => {
      unlistenRef.current?.();
      unlistenLicenses.then((fn) => fn());
      unlistenInstall.then((fn) => fn());
    };
  }, []);

  async function recheckTree() {
    const t = await invoke<boolean>("check_tree");
    setTreeInstalled(t);
  }

  async function handleStartScan() {
    if (!selectedDrive) return;
    setScreen("scanning");
    setProgress({ lines: 0, size_bytes: 0, elapsed_secs: 0, recent_entry: "" });
    setScanLog([]);
    setError(null);

    unlistenRef.current = await listen<ScanProgress>("scan-progress", (e) => {
      setProgress(e.payload);
      const entry = e.payload.recent_entry;
      if (entry) {
        setScanLog(prev => [...prev.slice(-7), entry]);
      }
    });

    try {
      const r = await invoke<ScanResult>("start_scan", { targetPath: selectedDrive.path });
      setResult(r);
      setScreen("complete");
    } catch (err) {
      setError(String(err));
      setScreen("select");
    } finally {
      unlistenRef.current?.();
      unlistenRef.current = null;
    }
  }

  function goToMenu() {
    setScreen("menu");
    setResult(null);
    setSelectedDrive(null);
    setProgress({ lines: 0, size_bytes: 0, elapsed_secs: 0, recent_entry: "" });
    setScanLog([]);
    setError(null);
  }

  // fullHeight: content starts from top (no vertical centering) for list/form screens
  const fullHeight = screen === "menu" || screen === "select" || screen === "logs" || screen === "licenses";
  // Header is always compact on non-menu screens — hides the large TreeIcon so it
  // doesn't dominate the layout while scanning/complete content is centered below
  const compactHeader = true;

  return (
    <div
      className="h-screen bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 flex flex-col overflow-hidden"
      style={{ fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace" }}
    >
      {/* Native titlebar drag region — blends with app background, keeps traffic lights */}
      <div
        data-tauri-drag-region
        className="h-7 w-full shrink-0 select-none bg-slate-50 dark:bg-slate-950"
      />

      {/* Main content */}
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden px-6 pb-4">
        {screen !== "menu" && <Header systemInfo={systemInfo} compact={compactHeader} />}

        <div className={`flex-1 min-h-0 flex flex-col items-center w-full ${fullHeight ? "" : "justify-center"}`}>
        {screen === "menu" && (
          <MenuScreen
            systemInfo={systemInfo}
            treeInstalled={treeInstalled}
            onCreateSnapshot={() => setScreen("select")}
            onViewLogs={() => setScreen("logs")}
            onOfflineClick={() => setShowInstallDialog(true)}
          />
        )}
        {screen === "select" && (
          <SelectScreen
            drives={drives}
            treeInstalled={treeInstalled}
            selectedDrive={selectedDrive}
            onSelectDrive={setSelectedDrive}
            onStartScan={handleStartScan}
            onBack={goToMenu}
            error={error}
          />
        )}
        {screen === "scanning" && (
          <ScanningScreen progress={progress} drivePath={selectedDrive?.path ?? ""} scanLog={scanLog} />
        )}
        {screen === "complete" && result && (
          <CompleteScreen result={result} onDone={goToMenu} />
        )}
        {screen === "logs" && (
          <LogsScreen onBack={goToMenu} />
        )}
        {screen === "licenses" && (
          <LicensesScreen onBack={goToMenu} />
        )}
        </div>{/* end screen area */}
      </div>{/* end main content */}

      {showInstallDialog && (
        <InstallDialog
          osLabel={systemInfo?.os_label ?? ""}
          onClose={() => setShowInstallDialog(false)}
          onInstalled={() => {
            setShowInstallDialog(false);
            recheckTree();
          }}
        />
      )}
    </div>
  );
}

// ── Header ────────────────────────────────────────────────────────────────────

function Header({ systemInfo, compact }: { systemInfo: SystemInfo | null; compact: boolean }) {
  return (
    <header className={`text-center select-none shrink-0 ${compact ? "mb-4" : "mb-8"}`}>
      {!compact && <TreeIcon />}
      <h1 className={`font-bold text-emerald-600 dark:text-emerald-400 tracking-widest uppercase ${compact ? "text-base" : "text-xl mt-3"}`}>
        TreeSnapshots
      </h1>
      {!compact && <p className="text-slate-500 dark:text-slate-600 text-xs mt-1">File System Snapshot Tool</p>}
      {systemInfo && (
        <p className="text-slate-400 dark:text-slate-700 text-xs mt-1.5 tracking-wide">
          {systemInfo.os_label}
          <span className="mx-1.5 text-slate-300 dark:text-slate-800">·</span>
          {systemInfo.hostname}
          <span className="mx-1.5 text-slate-300 dark:text-slate-800">·</span>
          {systemInfo.username}
        </p>
      )}
    </header>
  );
}

// ── Menu Screen ───────────────────────────────────────────────────────────────

function MenuScreen({
  systemInfo,
  treeInstalled,
  onCreateSnapshot,
  onViewLogs,
  onOfflineClick,
}: {
  systemInfo: SystemInfo | null;
  treeInstalled: boolean | null;
  onCreateSnapshot: () => void;
  onViewLogs: () => void;
  onOfflineClick: () => void;
}) {
  return (
    <div className="w-full max-w-md flex flex-col flex-1 min-h-0 items-center">
      <div className="flex-1 flex flex-col items-center justify-center gap-8 w-full">
        {/* Tree + title + system info */}
        <div className="text-center select-none">
          <TreeIcon />
          <h1 className="text-xl font-bold text-emerald-600 dark:text-emerald-400 tracking-widest uppercase mt-3">
            TreeSnapshots
          </h1>
          <p className="text-slate-500 dark:text-slate-600 text-xs mt-1">File System Snapshot Tool</p>
          {treeInstalled !== null && (
            treeInstalled ? (
              <div className="flex items-center justify-center gap-1.5 mt-2 select-none">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                <span className="text-xs tracking-widest uppercase text-emerald-600 dark:text-emerald-800">online</span>
              </div>
            ) : (
              <button
                onClick={onOfflineClick}
                className="flex items-center justify-center gap-1.5 mt-2 cursor-pointer group"
              >
                <span className="w-1.5 h-1.5 rounded-full bg-red-500" />
                <span className="text-xs tracking-widest uppercase text-red-500 dark:text-red-800 group-hover:text-red-600 dark:group-hover:text-red-500 transition-colors duration-100">
                  offline — click to install
                </span>
              </button>
            )
          )}
          {systemInfo && (
            <p className="text-slate-400 dark:text-slate-700 text-xs mt-2 tracking-wide">
              {systemInfo.os_label}
              <span className="mx-1.5 text-slate-300 dark:text-slate-800">·</span>
              {systemInfo.hostname}
              <span className="mx-1.5 text-slate-300 dark:text-slate-800">·</span>
              {systemInfo.username}
            </p>
          )}
        </div>

        {/* Buttons */}
        <div className="w-full space-y-3">
          <MenuButton
            icon={<IconCamera className="w-6 h-6" />}
            title="Create Snapshot"
            subtitle="Capture your file system tree"
            onClick={onCreateSnapshot}
          />
          <MenuButton
            icon={<IconClipboard className="w-6 h-6" />}
            title="View Snapshot Logs"
            subtitle="Browse previously saved snapshots"
            onClick={onViewLogs}
          />
        </div>
      </div>

      <p className="text-slate-300 dark:text-slate-800 text-xs select-none pb-1">© 2026 leejongyoung</p>
    </div>
  );
}

function MenuButton({
  icon, title, subtitle, onClick,
}: {
  icon: React.ReactNode; title: string; subtitle: string; onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="w-full flex items-center gap-4 px-5 py-4 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 hover:border-emerald-400 dark:hover:border-emerald-800 hover:bg-slate-50 dark:hover:bg-slate-800 rounded-xl transition-all duration-100 cursor-pointer group"
    >
      <span className="text-slate-400 dark:text-slate-500 group-hover:text-emerald-600 dark:group-hover:text-emerald-600 transition-colors duration-100 shrink-0">{icon}</span>
      <div className="flex-1 text-left">
        <p className="text-slate-800 dark:text-slate-100 font-semibold text-sm group-hover:text-emerald-600 dark:group-hover:text-emerald-300 transition-colors duration-100">
          {title}
        </p>
        <p className="text-slate-400 dark:text-slate-600 text-xs mt-0.5">{subtitle}</p>
      </div>
      <span className="text-slate-300 dark:text-slate-700 group-hover:text-emerald-500 dark:group-hover:text-emerald-600 text-lg transition-colors duration-100">›</span>
    </button>
  );
}

// ── Select Screen ─────────────────────────────────────────────────────────────

function SelectScreen({
  drives, treeInstalled, selectedDrive, onSelectDrive, onStartScan, onBack, error,
}: {
  drives: Drive[];
  treeInstalled: boolean | null;
  selectedDrive: Drive | null;
  onSelectDrive: (d: Drive) => void;
  onStartScan: () => void;
  onBack: () => void;
  error: string | null;
}) {
  const canStart = !!selectedDrive && treeInstalled !== false;

  return (
    <div className="w-full max-w-md flex flex-col flex-1 min-h-0">
      <BackButton onClick={onBack} />

      {treeInstalled === false && (
        <div className="mb-3 px-4 py-3 bg-amber-50 dark:bg-amber-950 border border-amber-300 dark:border-amber-700 rounded-lg text-amber-700 dark:text-amber-300 text-xs leading-relaxed shrink-0">
          <span className="font-bold">⚠ tree not found.</span>
          <br />
          macOS: <code className="text-amber-600 dark:text-amber-400">brew install tree</code>
          {"  "}/ Linux: <code className="text-amber-600 dark:text-amber-400">apt install tree</code>
        </div>
      )}

      {error && (
        <div className="mb-3 px-4 py-3 bg-red-50 dark:bg-red-950 border border-red-300 dark:border-red-700 rounded-lg text-red-600 dark:text-red-300 text-xs shrink-0">
          <span className="font-bold">Error: </span>{error}
        </div>
      )}

      <p className="text-slate-500 text-xs mb-2 uppercase tracking-wider shrink-0">
        Select a drive to snapshot
        {drives.length > 0 && (
          <span className="ml-2 text-slate-300 dark:text-slate-700">({drives.length})</span>
        )}
      </p>

      {/* Scrollable drive list */}
      <div className="flex-1 overflow-y-auto min-h-0 space-y-2 mb-3 pr-1">
        {drives.length === 0 ? (
          <p className="text-slate-400 dark:text-slate-700 text-sm text-center py-6">Detecting drives…</p>
        ) : (
          drives.map((drive) => {
            const isSelected = selectedDrive?.path === drive.path;
            return (
              <button
                key={drive.path}
                onClick={() => onSelectDrive(drive)}
                className={[
                  "w-full text-left px-4 py-3 rounded-lg border transition-all duration-100",
                  "flex items-center gap-3 cursor-pointer",
                  isSelected
                    ? "bg-emerald-50 dark:bg-emerald-950 border-emerald-400 dark:border-emerald-600 ring-pulse"
                    : "bg-white dark:bg-slate-900 border-slate-200 dark:border-slate-800 hover:border-slate-400 dark:hover:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-800",
                ].join(" ")}
              >
                <DriveIconSvg path={drive.path} selected={isSelected} />
                <div className="flex-1 min-w-0">
                  <p className={`text-sm font-medium truncate ${isSelected ? "text-emerald-600 dark:text-emerald-300" : "text-slate-700 dark:text-slate-200"}`}>
                    {drive.label}
                  </p>
                  <p className="text-xs text-slate-400 dark:text-slate-600 mt-0.5 truncate">{drive.path}</p>
                </div>
                {isSelected && (
                  <span className="text-emerald-500 text-xs font-bold shrink-0">✓</span>
                )}
              </button>
            );
          })
        )}
      </div>

      {/* Start button */}
      <button
        onClick={onStartScan}
        disabled={!canStart}
        className={[
          "w-full py-3 rounded-lg font-bold text-xs tracking-widest uppercase transition-all duration-100 shrink-0",
          canStart
            ? "bg-emerald-600 dark:bg-emerald-700 hover:bg-emerald-500 dark:hover:bg-emerald-600 text-white cursor-pointer"
            : "bg-slate-100 dark:bg-slate-900 text-slate-300 dark:text-slate-700 cursor-not-allowed border border-slate-200 dark:border-slate-800",
        ].join(" ")}
      >
        Start Snapshot
      </button>

      <p className="text-slate-300 dark:text-slate-800 text-xs text-center select-none mt-2 shrink-0">© 2026 leejongyoung</p>
    </div>
  );
}

// ── Scanning Screen ───────────────────────────────────────────────────────────

function ScanningScreen({ progress, drivePath, scanLog }: {
  progress: ScanProgress;
  drivePath: string;
  scanLog: string[];
}) {
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "instant" });
  }, [scanLog]);

  return (
    <div className="w-full max-w-md text-center">
      <div className="flex justify-center gap-2 mb-6">
        <span className="w-2.5 h-2.5 bg-emerald-500 dark:bg-emerald-400 rounded-full dot-1" />
        <span className="w-2.5 h-2.5 bg-emerald-500 dark:bg-emerald-400 rounded-full dot-2" />
        <span className="w-2.5 h-2.5 bg-emerald-500 dark:bg-emerald-400 rounded-full dot-3" />
      </div>
      <p className="text-emerald-600 dark:text-emerald-400 font-bold text-sm mb-1">Scanning</p>
      <p className="text-slate-400 dark:text-slate-600 text-xs mb-6 truncate px-4">{drivePath}</p>

      <div className="grid grid-cols-3 gap-3 mb-5">
        <StatBox label="Lines" value={progress.lines.toLocaleString()} />
        <StatBox label="Size" value={formatBytes(progress.size_bytes)} />
        <StatBox label="Elapsed" value={formatDuration(progress.elapsed_secs)} />
      </div>

      {/* Live entry log */}
      <div className="rounded-lg bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 overflow-hidden">
        <div className="h-32 overflow-y-auto px-3 py-2 space-y-0.5 text-left">
          {scanLog.length === 0 ? (
            <p className="text-slate-300 dark:text-slate-700 text-xs">Waiting...</p>
          ) : (
            scanLog.map((entry, i) => (
              <p
                key={i}
                className={`text-xs truncate ${
                  i === scanLog.length - 1
                    ? "text-slate-600 dark:text-slate-400"
                    : "text-slate-300 dark:text-slate-700"
                }`}
              >
                <span className="text-slate-300 dark:text-slate-700 mr-1 select-none">└</span>
                {entry}
              </p>
            ))
          )}
          <div ref={logEndRef} />
        </div>
      </div>
    </div>
  );
}

// ── Complete Screen ───────────────────────────────────────────────────────────

function CompleteScreen({ result, onDone }: { result: ScanResult; onDone: () => void }) {
  return (
    <div className="w-full max-w-md text-center">
      <div className="text-3xl mb-3 text-emerald-600 dark:text-emerald-400">✓</div>
      <p className="text-emerald-600 dark:text-emerald-400 font-bold text-sm mb-1">Snapshot Complete</p>
      <p className="text-slate-500 dark:text-slate-600 text-xs mb-7">File system tree captured successfully</p>
      <div className="grid grid-cols-3 gap-3 mb-5">
        <StatBox label="Lines" value={result.total_lines.toLocaleString()} />
        <StatBox label="Size" value={formatBytes(result.total_size_bytes)} />
        <StatBox label="Duration" value={formatDuration(result.duration_secs)} />
      </div>
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-lg px-4 py-3 mb-5 text-left">
        <p className="text-slate-400 dark:text-slate-600 text-xs uppercase tracking-wider mb-1.5">Saved to</p>
        <p className="text-emerald-600 dark:text-emerald-400 text-xs break-all leading-relaxed">{result.file_path}</p>
      </div>
      <button
        onClick={onDone}
        className="w-full py-3 rounded-lg font-bold text-xs tracking-widest uppercase bg-white dark:bg-slate-900 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 text-slate-500 dark:text-slate-400 transition-all duration-100 cursor-pointer"
      >
        Done
      </button>
    </div>
  );
}

// ── Logs Screen ───────────────────────────────────────────────────────────────

function LogsScreen({ onBack }: { onBack: () => void }) {
  const [logs, setLogs] = useState<SnapshotLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectMode, setSelectMode] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);

  function loadLogs() {
    setLoading(true);
    invoke<SnapshotLog[]>("get_snapshot_logs")
      .then(setLogs)
      .finally(() => setLoading(false));
  }

  useEffect(() => { loadLogs(); }, []);

  function enterSelectMode() {
    setSelectMode(true);
    setSelected(new Set());
    setConfirming(false);
  }

  function exitSelectMode() {
    setSelectMode(false);
    setSelected(new Set());
    setConfirming(false);
  }

  function toggleItem(path: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(path) ? next.delete(path) : next.add(path);
      return next;
    });
    setConfirming(false);
  }

  function toggleAll() {
    if (selected.size === logs.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(logs.map((l) => l.file_path)));
    }
    setConfirming(false);
  }

  async function deleteSelected() {
    await Promise.all(
      [...selected].map((path) => invoke("delete_snapshot_log", { filePath: path }))
    );
    exitSelectMode();
    loadLogs();
  }

  const allSelected = logs.length > 0 && selected.size === logs.length;

  return (
    <div className="w-full max-w-md flex flex-col flex-1 min-h-0">
      {/* Header row */}
      <div className="flex items-center justify-between mb-3 shrink-0">
        {selectMode ? (
          <button
            onClick={exitSelectMode}
            className="text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 text-xs transition-colors duration-100 cursor-pointer"
          >
            Cancel
          </button>
        ) : (
          <BackButton onClick={onBack} />
        )}
        <div className="flex items-center gap-3">
          {!selectMode && (
            <button
              onClick={loadLogs}
              className="text-slate-400 dark:text-slate-600 hover:text-slate-600 dark:hover:text-slate-400 text-xs transition-colors duration-100 cursor-pointer"
            >
              ↻ Refresh
            </button>
          )}
          {logs.length > 0 && (
            <button
              onClick={selectMode ? toggleAll : enterSelectMode}
              className="text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 text-xs transition-colors duration-100 cursor-pointer"
            >
              {selectMode ? (allSelected ? "Deselect All" : "Select All") : "Select"}
            </button>
          )}
        </div>
      </div>

      {/* Label row */}
      <p className="text-slate-500 text-xs mb-2 uppercase tracking-wider shrink-0">
        Snapshot Logs
        {logs.length > 0 && (
          <span className="ml-2 text-slate-300 dark:text-slate-700">({logs.length})</span>
        )}
      </p>

      {/* Scrollable log list */}
      <div className="flex-1 overflow-y-auto min-h-0 space-y-2 pr-1">
        {loading ? (
          <p className="text-slate-400 dark:text-slate-700 text-sm text-center py-8">Loading…</p>
        ) : logs.length === 0 ? (
          <div className="text-center py-10">
            <p className="text-slate-500 dark:text-slate-600 text-sm">No snapshots found.</p>
            <p className="text-slate-400 dark:text-slate-700 text-xs mt-1">Create your first snapshot to see it here.</p>
          </div>
        ) : (
          logs.map((log) => {
            const isChecked = selected.has(log.file_path);
            return (
              <button
                key={log.file_path}
                onClick={() =>
                  selectMode
                    ? toggleItem(log.file_path)
                    : invoke("open_file", { filePath: log.file_path })
                }
                className={[
                  "w-full text-left px-4 py-3 rounded-lg border transition-all duration-100 cursor-pointer group",
                  "flex items-center gap-3",
                  isChecked
                    ? "bg-red-50 dark:bg-red-950 border-red-200 dark:border-red-800"
                    : "bg-white dark:bg-slate-900 border-slate-200 dark:border-slate-800 hover:border-emerald-400 dark:hover:border-emerald-800 hover:bg-slate-50 dark:hover:bg-slate-800",
                ].join(" ")}
              >
                {/* File icon or Checkbox */}
                {!selectMode && (
                  <span className={isChecked ? "text-red-400" : "text-slate-300 dark:text-slate-700 group-hover:text-slate-400 dark:group-hover:text-slate-500 transition-colors duration-100"}>
                    <IconFile className="w-4 h-4" />
                  </span>
                )}
                {selectMode && (
                  <span
                    className={[
                      "shrink-0 w-4 h-4 rounded border flex items-center justify-center text-xs transition-all duration-100",
                      isChecked
                        ? "bg-red-600 border-red-500 text-white"
                        : "border-slate-300 dark:border-slate-600",
                    ].join(" ")}
                  >
                    {isChecked && "✓"}
                  </span>
                )}
                <div className="flex-1 min-w-0">
                  <p className={[
                    "text-xs font-medium truncate transition-colors duration-100",
                    isChecked
                      ? "text-red-500 dark:text-red-300"
                      : "text-emerald-600 dark:text-emerald-400 group-hover:text-emerald-500 dark:group-hover:text-emerald-300",
                  ].join(" ")}>
                    {parseLogFilename(log.filename)}
                  </p>
                  <div className="flex items-center justify-between mt-1.5">
                    <p className="text-slate-400 dark:text-slate-600 text-xs">{log.modified_at}</p>
                    <p className="text-slate-400 dark:text-slate-600 text-xs">{formatBytes(log.size_bytes)}</p>
                  </div>
                </div>
              </button>
            );
          })
        )}
      </div>

      {/* Footer */}
      {!selectMode && (
        <div className="mt-3 pt-3 border-t border-slate-100 dark:border-slate-900 text-center shrink-0 select-none">
          <p className="text-slate-300 dark:text-slate-800 text-xs">© 2026 leejongyoung</p>
        </div>
      )}

      {/* Delete controls */}
      {selectMode && (
        <div className="mt-3 shrink-0">
          {confirming ? (
            <div className="flex gap-2">
              <button
                onClick={() => setConfirming(false)}
                className="flex-1 py-3 rounded-lg font-bold text-xs tracking-widest uppercase bg-white dark:bg-slate-900 hover:bg-slate-100 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800 text-slate-500 dark:text-slate-400 transition-all duration-100 cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={deleteSelected}
                className="flex-1 py-3 rounded-lg font-bold text-xs tracking-widest uppercase bg-red-600 dark:bg-red-700 hover:bg-red-500 dark:hover:bg-red-600 border border-red-500 dark:border-red-600 text-white transition-all duration-100 cursor-pointer"
              >
                Confirm Delete
              </button>
            </div>
          ) : (
            <button
              onClick={() => selected.size > 0 && setConfirming(true)}
              disabled={selected.size === 0}
              className={[
                "w-full py-3 rounded-lg font-bold text-xs tracking-widest uppercase transition-all duration-100",
                selected.size > 0
                  ? "bg-red-100 dark:bg-red-900 hover:bg-red-200 dark:hover:bg-red-800 border border-red-300 dark:border-red-700 text-red-600 dark:text-red-200 cursor-pointer"
                  : "bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-slate-300 dark:text-slate-700 cursor-not-allowed",
              ].join(" ")}
            >
              {selected.size > 0 ? `Delete (${selected.size})` : "Select items to delete"}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

// ── Shared ────────────────────────────────────────────────────────────────────

function StatBox({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-lg px-3 py-3">
      <p className="text-slate-400 dark:text-slate-600 text-xs uppercase tracking-wider mb-1">{label}</p>
      <p className="text-slate-900 dark:text-slate-100 font-bold text-sm">{value}</p>
    </div>
  );
}

function BackButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-1.5 text-slate-400 dark:text-slate-600 hover:text-slate-600 dark:hover:text-slate-400 text-xs mb-2 transition-colors duration-100 cursor-pointer"
    >
      ‹ Back
    </button>
  );
}

// ── Licenses Screen ───────────────────────────────────────────────────────────

const OSS_LICENSES: { name: string; version: string; license: string; url: string; role: string }[] = [
  { name: "Tauri",               version: "2.10.3", license: "MIT / Apache 2.0", url: "https://tauri.app",                                 role: "Desktop app framework" },
  { name: "React",               version: "19.2.4", license: "MIT",              url: "https://react.dev",                                 role: "UI library" },
  { name: "Tailwind CSS",        version: "4.2.2",  license: "MIT",              url: "https://tailwindcss.com",                           role: "Utility-first CSS" },
  { name: "Vite",                version: "7.3.1",  license: "MIT",              url: "https://vitejs.dev",                                role: "Frontend build tool" },
  { name: "TypeScript",          version: "5.8.3",  license: "Apache 2.0",       url: "https://www.typescriptlang.org",                    role: "Typed JavaScript" },
  { name: "serde",               version: "1.0.228", license: "MIT / Apache 2.0", url: "https://serde.rs",                                 role: "Rust serialization" },
  { name: "chrono",              version: "0.4.44", license: "MIT / Apache 2.0", url: "https://github.com/chronotope/chrono",              role: "Rust date & time" },
  { name: "tauri-plugin-opener", version: "2.5.3",  license: "MIT / Apache 2.0", url: "https://github.com/tauri-apps/plugins-workspace",   role: "File & URL opener" },
];

function LicensesScreen({ onBack }: { onBack: () => void }) {
  return (
    <div className="w-full max-w-md flex flex-col flex-1 min-h-0">
      <BackButton onClick={onBack} />

      <p className="text-slate-500 text-xs mb-2 uppercase tracking-wider shrink-0">
        Open Source Licenses
      </p>

      <div className="flex-1 overflow-y-auto min-h-0 space-y-2 pr-1">
        {OSS_LICENSES.map((lib) => (
          <button
            key={lib.name}
            onClick={() => invoke("open_url_external", { url: lib.url }).catch(() => {})}
            className="w-full text-left px-4 py-3 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800 rounded-lg transition-all duration-100 cursor-pointer group"
          >
            <div className="flex items-center justify-between mb-1">
              <span className="text-emerald-600 dark:text-emerald-400 text-xs font-semibold group-hover:text-emerald-500 dark:group-hover:text-emerald-300 transition-colors duration-100">
                {lib.name}
              </span>
              <span className="text-slate-300 dark:text-slate-700 text-xs">{lib.version}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-slate-400 dark:text-slate-600 text-xs">{lib.role}</span>
              <span className="text-slate-300 dark:text-slate-700 text-xs">{lib.license}</span>
            </div>
          </button>
        ))}
      </div>

      <div className="mt-3 pt-3 border-t border-slate-100 dark:border-slate-900 text-center shrink-0 select-none">
        <p className="text-slate-300 dark:text-slate-800 text-xs">© 2026 leejongyoung</p>
      </div>
    </div>
  );
}

// ── Tree SVG ──────────────────────────────────────────────────────────────────

function TreeIcon() {
  return (
    <svg
      width="52"
      height="66"
      viewBox="0 0 52 66"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className="mx-auto"
    >
      <rect x="21" y="52" width="10" height="13" rx="2" fill="#94a3b8" />
      <polygon points="26,18 1,56 51,56" fill="#065f46" />
      <polygon points="26,9 5,44 47,44" fill="#047857" />
      <polygon points="26,1 11,32 41,32" fill="#10b981" />
      <circle cx="26" cy="1" r="3.5" fill="#6ee7b7" opacity="0.85" />
    </svg>
  );
}

// ── Install Dialog ────────────────────────────────────────────────────────────

type InstallPhase = "confirm" | "installing" | "done";

function InstallDialog({
  osLabel,
  onClose,
  onInstalled,
}: {
  osLabel: string;
  onClose: () => void;
  onInstalled: () => void;
}) {
  const [phase, setPhase] = useState<InstallPhase>("confirm");
  const [lines, setLines] = useState<string[]>([]);
  const [success, setSuccess] = useState(false);
  const [errMsg, setErrMsg] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  const manualCmd = osLabel.includes("macOS")
    ? "brew install tree"
    : osLabel.includes("Fedora")
    ? "sudo dnf install tree"
    : osLabel.includes("RHEL") || osLabel.includes("Rocky") || osLabel.includes("CentOS")
    ? "sudo yum install tree"
    : "sudo apt install tree";

  useEffect(() => {
    const unsub = listen<string>("install-output", (e) => {
      setLines((prev) => [...prev, e.payload]);
    });
    return () => { unsub.then((fn) => fn()); };
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "instant" });
  }, [lines]);

  async function startInstall() {
    setPhase("installing");
    setLines([]);
    setErrMsg("");
    try {
      await invoke("install_tree");
      setSuccess(true);
      onInstalled();
    } catch (e) {
      setErrMsg(String(e));
      setSuccess(false);
    }
    setPhase("done");
  }

  return (
    <div
      className="fixed inset-0 bg-black/70 flex items-center justify-center z-50 px-6"
      style={{ fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace" }}
    >
      <div className="w-full max-w-md bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-xl overflow-hidden shadow-xl">

        {/* ── Confirm ── */}
        {phase === "confirm" && (
          <div className="p-6">
            <div className="flex items-center gap-2 mb-4">
              <span className="w-2 h-2 rounded-full bg-red-500 shrink-0" />
              <h2 className="text-red-500 dark:text-red-400 font-bold text-sm tracking-widest uppercase">tree not found</h2>
            </div>
            <p className="text-slate-700 dark:text-slate-300 text-sm mb-1">
              The <code className="text-emerald-600 dark:text-emerald-400 bg-slate-100 dark:bg-slate-800 px-1 rounded">tree</code> command is required to create snapshots but is not installed.
            </p>
            <p className="text-slate-400 dark:text-slate-600 text-xs mb-4 mt-1">
              Install automatically, or run the command below in a terminal.
            </p>
            <div className="bg-slate-950 border border-slate-800 rounded-lg px-4 py-3 mb-5">
              <p className="text-slate-500 text-xs uppercase tracking-wider mb-1.5">Manual install</p>
              <p className="text-emerald-400 text-xs">{manualCmd}</p>
            </div>
            <div className="flex gap-3">
              <button
                onClick={onClose}
                className="flex-1 py-2.5 rounded-lg text-xs font-bold tracking-widest uppercase bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 border border-slate-200 dark:border-slate-700 text-slate-500 dark:text-slate-400 transition-all duration-100 cursor-pointer"
              >
                Later
              </button>
              <button
                onClick={startInstall}
                className="flex-1 py-2.5 rounded-lg text-xs font-bold tracking-widest uppercase bg-emerald-600 dark:bg-emerald-700 hover:bg-emerald-500 dark:hover:bg-emerald-600 text-white transition-all duration-100 cursor-pointer"
              >
                Install Now
              </button>
            </div>
          </div>
        )}

        {/* ── Installing ── */}
        {phase === "installing" && (
          <div className="p-6">
            <div className="flex items-center gap-2 mb-4">
              <span className="w-2 h-2 rounded-full bg-amber-500 dot-1 shrink-0" />
              <h2 className="text-amber-600 dark:text-amber-400 font-bold text-sm tracking-widest uppercase">Installing…</h2>
            </div>
            <div className="bg-slate-950 border border-slate-800 rounded-lg p-3 h-44 overflow-y-auto">
              {lines.length === 0 ? (
                <p className="text-slate-700 text-xs">Starting…</p>
              ) : (
                lines.map((l, i) => (
                  <p key={i} className="text-slate-400 text-xs leading-relaxed whitespace-pre-wrap">{l || "\u00a0"}</p>
                ))
              )}
              <div ref={bottomRef} />
            </div>
          </div>
        )}

        {/* ── Done: Success ── */}
        {phase === "done" && success && (
          <div className="p-6">
            <div className="flex items-center gap-2 mb-4">
              <span className="text-emerald-500 dark:text-emerald-400">✓</span>
              <h2 className="text-emerald-600 dark:text-emerald-400 font-bold text-sm tracking-widest uppercase">Installed</h2>
            </div>
            <p className="text-slate-700 dark:text-slate-300 text-sm mb-5">
              <code className="text-emerald-600 dark:text-emerald-400 bg-slate-100 dark:bg-slate-800 px-1 rounded">tree</code> is now available. You can create snapshots.
            </p>
            <button
              onClick={onClose}
              className="w-full py-2.5 rounded-lg text-xs font-bold tracking-widest uppercase bg-emerald-600 dark:bg-emerald-700 hover:bg-emerald-500 dark:hover:bg-emerald-600 text-white transition-all duration-100 cursor-pointer"
            >
              Done
            </button>
          </div>
        )}

        {/* ── Done: Error ── */}
        {phase === "done" && !success && (
          <div className="p-6">
            <div className="flex items-center gap-2 mb-4">
              <span className="text-red-500 dark:text-red-400">✕</span>
              <h2 className="text-red-500 dark:text-red-400 font-bold text-sm tracking-widest uppercase">Installation Failed</h2>
            </div>
            <p className="text-slate-500 text-xs mb-4 leading-relaxed">{errMsg}</p>
            <div className="bg-slate-950 border border-slate-800 rounded-lg px-4 py-3 mb-5">
              <p className="text-slate-500 text-xs uppercase tracking-wider mb-1.5">Install manually</p>
              <p className="text-emerald-400 text-xs">{manualCmd}</p>
            </div>
            <div className="flex gap-3">
              <button
                onClick={onClose}
                className="flex-1 py-2.5 rounded-lg text-xs font-bold tracking-widest uppercase bg-slate-100 dark:bg-slate-800 hover:bg-slate-200 dark:hover:bg-slate-700 border border-slate-200 dark:border-slate-700 text-slate-500 dark:text-slate-400 transition-all duration-100 cursor-pointer"
              >
                Close
              </button>
              <button
                onClick={startInstall}
                className="flex-1 py-2.5 rounded-lg text-xs font-bold tracking-widest uppercase bg-red-100 dark:bg-red-900 hover:bg-red-200 dark:hover:bg-red-800 border border-red-300 dark:border-red-700 text-red-600 dark:text-red-300 transition-all duration-100 cursor-pointer"
              >
                Retry
              </button>
            </div>
          </div>
        )}

      </div>
    </div>
  );
}

// ── SVG Icon Components ────────────────────────────────────────────────────────

function IconCamera({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14.5 4h-5l-2 3H4a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1V8a1 1 0 0 0-1-1h-3.5l-2-3Z" />
      <circle cx="12" cy="11" r="3" />
    </svg>
  );
}

function IconClipboard({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <rect x="5" y="3" width="14" height="18" rx="2" />
      <path d="M9 7h6M9 11h6M9 15h4" />
    </svg>
  );
}

function IconFile({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6Z" />
      <path d="M14 2v6h6M9 13h6M9 17h4" />
    </svg>
  );
}

function IconMonitor({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <path d="M8 21h8M12 17v4" />
    </svg>
  );
}

function IconWindowsGrid({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="7.5" height="7.5" rx="1" />
      <rect x="13.5" y="3" width="7.5" height="7.5" rx="1" />
      <rect x="3" y="13.5" width="7.5" height="7.5" rx="1" />
      <rect x="13.5" y="13.5" width="7.5" height="7.5" rx="1" />
    </svg>
  );
}

function IconHardDrive({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="7" width="20" height="10" rx="2" />
      <path d="M6 12h4" />
      <circle cx="17" cy="12" r="1.5" fill="currentColor" stroke="none" />
    </svg>
  );
}

function DriveIconSvg({ path, selected }: { path: string; selected: boolean }) {
  const cls = `w-5 h-5 shrink-0 transition-colors duration-100 ${selected ? "text-emerald-500" : "text-slate-400 dark:text-slate-600"}`;
  if (path === "/") return <IconMonitor className={cls} />;
  if (path.startsWith("/mnt/")) return <IconWindowsGrid className={cls} />;
  return <IconHardDrive className={cls} />;
}
