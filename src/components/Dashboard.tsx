import { formatBytesPerSec, formatEta } from "../api";
import type { FileActivity, SyncTriggerStatus, UploadProgress } from "../types";

interface Props {
  progress: UploadProgress;
  activities: FileActivity[];
  syncStatus: SyncTriggerStatus;
  onUpload: () => void;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
  onRetry: () => void;
}

function statusClass(status: string): string {
  return `status-badge status-${status}`;
}

export function Dashboard({
  progress,
  activities,
  syncStatus,
  onUpload,
  onPause,
  onResume,
  onCancel,
  onRetry,
}: Props) {
  const pct =
    progress.total_files > 0
      ? Math.round((progress.completed_files / progress.total_files) * 100)
      : progress.is_running
        ? 0
        : 0;

  return (
    <div className="dashboard">
      <header className="page-header">
        <div>
          <h1>Dashboard</h1>
          <p>Real-time upload progress and file activity</p>
        </div>
        <div className="header-actions">
          <button className="btn primary" onClick={onUpload}>
            Upload Files
          </button>
          {progress.is_running && !progress.is_paused && (
            <button className="btn" onClick={onPause}>
              Pause
            </button>
          )}
          {progress.is_paused && (
            <button className="btn" onClick={onResume}>
              Resume
            </button>
          )}
          {progress.is_running && (
            <button className="btn danger" onClick={onCancel}>
              Cancel
            </button>
          )}
          <button className="btn" onClick={onRetry}>
            Retry Failed
          </button>
        </div>
      </header>

      {!syncStatus.can_sync && (
        <div className="alert warning">
          Sync paused: {syncStatus.reasons.join(" · ")}
        </div>
      )}

      <div className="stats-grid">
        <div className="stat-card">
          <span className="stat-label">Progress</span>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${pct}%` }} />
          </div>
          <span className="stat-value">
            {progress.completed_files} / {progress.total_files || "—"} files
          </span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Speed</span>
          <span className="stat-value big">
            {formatBytesPerSec(progress.bytes_per_second)}
          </span>
        </div>
        <div className="stat-card">
          <span className="stat-label">ETA</span>
          <span className="stat-value big">
            {formatEta(progress.eta_seconds)}
          </span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Failed / Skipped</span>
          <span className="stat-value">
            {progress.failed_files} / {progress.skipped_files}
          </span>
        </div>
      </div>

      {progress.current_file && (
        <div className="current-file">
          Uploading: <code>{progress.current_file}</code>
        </div>
      )}

      <section className="activity-section">
        <h2>File Activity</h2>
        <div className="activity-list">
          {activities.length === 0 ? (
            <p className="empty">No activity yet. Start an upload to see progress here.</p>
          ) : (
            activities.slice(0, 100).map((a) => (
              <div key={a.id} className="activity-row">
                <span className={statusClass(a.status)}>{a.status}</span>
                <span className="activity-path" title={a.path}>
                  {a.path}
                </span>
                <span className="activity-time">
                  {new Date(a.timestamp).toLocaleTimeString()}
                </span>
              </div>
            ))
          )}
        </div>
      </section>

      <section className="triggers-section">
        <h2>Sync Triggers</h2>
        <div className="trigger-grid">
          <TriggerItem label="Wi-Fi" ok={syncStatus.wifi_connected} />
          <TriggerItem label="Allowed Network" ok={syncStatus.on_allowed_network} />
          <TriggerItem label="Plugged In" ok={syncStatus.plugged_in} />
          <TriggerItem label="Schedule" ok={syncStatus.within_schedule} />
        </div>
      </section>
    </div>
  );
}

function TriggerItem({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div className={`trigger-item ${ok ? "ok" : "blocked"}`}>
      <span>{label}</span>
      <span>{ok ? "✓" : "✗"}</span>
    </div>
  );
}
