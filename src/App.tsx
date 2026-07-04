import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Dashboard } from "./components/Dashboard";
import { SettingsPanel } from "./components/SettingsPanel";
import { SetupWizard } from "./components/SetupWizard";
import type {
  AppConfig,
  ConflictItem,
  FileActivity,
  RetryQueue,
  SyncTriggerStatus,
  UploadProgress,
} from "./types";
import { defaultConfig } from "./types";
import "./App.css";

type Tab = "dashboard" | "settings";

function App() {
  const [ready, setReady] = useState(false);
  const [config, setConfig] = useState<AppConfig>(defaultConfig());
  const [tab, setTab] = useState<Tab>("dashboard");
  const [progress, setProgress] = useState<UploadProgress>({
    total_files: 0,
    completed_files: 0,
    failed_files: 0,
    skipped_files: 0,
    bytes_per_second: 0,
    is_running: false,
    is_paused: false,
  });
  const [activities, setActivities] = useState<FileActivity[]>([]);
  const [syncStatus, setSyncStatus] = useState<SyncTriggerStatus>({
    can_sync: true,
    wifi_connected: true,
    on_allowed_network: true,
    plugged_in: true,
    within_schedule: true,
    reasons: [],
  });
  const [retryQueue, setRetryQueue] = useState<RetryQueue>({
    items: [],
    max_attempts: 5,
  });
  const [conflicts, setConflicts] = useState<ConflictItem[]>([]);

  const refresh = useCallback(async () => {
    const [p, a, s, r, c] = await Promise.all([
      api.getProgress(),
      api.getActivities(),
      api.getSyncStatus(),
      api.getRetryQueue(),
      api.getConflicts(),
    ]);
    setProgress(p);
    setActivities(a);
    setSyncStatus(s);
    setRetryQueue(r);
    setConflicts(c.conflicts);
  }, []);

  useEffect(() => {
    api.getConfig().then((cfg) => {
      setConfig(cfg);
      setReady(true);
    });
  }, []);

  useEffect(() => {
    if (!config.setup_complete) return;
    refresh();
    const id = setInterval(refresh, 1500);
    return () => clearInterval(id);
  }, [config.setup_complete, refresh]);

  const handleUpload = async () => {
    const paths = await api.pickUploadPaths();
    if (paths.length > 0) {
      await api.startUpload(paths);
      refresh();
    }
  };

  if (!ready) {
    return <div className="loading">Loading…</div>;
  }

  if (!config.setup_complete) {
    return (
      <SetupWizard
        onComplete={async () => {
          const cfg = await api.getConfig();
          setConfig(cfg);
        }}
      />
    );
  }

  return (
    <div className="app-shell">
      <nav className="sidebar">
        <div className="brand">
          <span className="brand-icon">📷</span>
          <div>
            <strong>Immich Desktop</strong>
            <small>CLI Wrapper</small>
          </div>
        </div>
        <button
          className={tab === "dashboard" ? "nav-btn active" : "nav-btn"}
          onClick={() => setTab("dashboard")}
        >
          Dashboard
        </button>
        <button
          className={tab === "settings" ? "nav-btn active" : "nav-btn"}
          onClick={() => setTab("settings")}
        >
          Settings
        </button>
        <div className="sidebar-footer">
          <button className="nav-btn" onClick={() => api.openLogsFolder()}>
            View Logs
          </button>
          {config.server_url && (
            <a
              className="nav-link"
              href={config.server_url.replace("/api", "")}
              target="_blank"
              rel="noreferrer"
            >
              Open Web UI
            </a>
          )}
        </div>
      </nav>

      <main className="main-content">
        {tab === "dashboard" ? (
          <Dashboard
            progress={progress}
            activities={activities}
            syncStatus={syncStatus}
            onUpload={handleUpload}
            onPause={() => api.pauseUpload().then(refresh)}
            onResume={() => api.resumeUpload().then(refresh)}
            onCancel={() => api.cancelUpload().then(refresh)}
            onRetry={() => api.retryFailed().then(refresh)}
          />
        ) : (
          <SettingsPanel
            config={config}
            retryQueue={retryQueue}
            conflicts={conflicts}
            onSave={setConfig}
            onRefresh={refresh}
          />
        )}
      </main>
    </div>
  );
}

export default App;
