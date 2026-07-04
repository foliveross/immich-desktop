import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  ConflictStore,
  DiscoveredServer,
  FileActivity,
  HandshakeResult,
  RetryQueue,
  ServerInfo,
  SyncTriggerStatus,
  UploadProgress,
} from "./types";

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<void>("save_app_config", { config }),
  getConfigPath: () => invoke<string>("get_config_path"),
  getLogsDir: () => invoke<string>("get_logs_dir"),
  getLockFilePath: () => invoke<string>("get_lock_file_path"),
  hasCredentials: () => invoke<boolean>("has_stored_credentials"),
  clearCredentials: () => invoke<void>("clear_credentials"),
  discoverServers: () => invoke<DiscoveredServer[]>("discover_servers"),
  performHandshake: (serverUrl: string, apiKey: string) =>
    invoke<HandshakeResult>("perform_handshake", { serverUrl, apiKey }),
  finalizeSetup: (serverUrl: string, apiKey: string) =>
    invoke<ServerInfo>("finalize_setup", { serverUrl, apiKey }),
  completeSetup: (serverUrl: string, apiKey: string) =>
    invoke<ServerInfo>("complete_setup", { serverUrl, apiKey }),
  testConnection: (serverUrl?: string, apiKey?: string) =>
    invoke<ServerInfo>("test_connection", { serverUrl, apiKey }),
  detectCli: () => invoke<string>("detect_cli"),
  startUpload: (paths: string[]) => invoke<void>("start_upload", { paths }),
  getProgress: () => invoke<UploadProgress>("get_upload_progress"),
  getActivities: () => invoke<FileActivity[]>("get_file_activities"),
  pauseUpload: () => invoke<void>("pause_upload"),
  resumeUpload: () => invoke<void>("resume_upload"),
  cancelUpload: () => invoke<void>("cancel_upload"),
  getSyncStatus: () => invoke<SyncTriggerStatus>("get_sync_trigger_status"),
  getCurrentNetwork: () => invoke<string | null>("get_current_network"),
  getRetryQueue: () => invoke<RetryQueue>("get_retry_queue"),
  removeFromRetryQueue: (id: string) =>
    invoke<RetryQueue>("remove_from_retry_queue", { id }),
  retryFailed: () => invoke<void>("retry_failed_uploads"),
  getConflicts: () => invoke<ConflictStore>("get_conflicts"),
  resolveConflict: (id: string, resolution: string) =>
    invoke<ConflictStore>("resolve_conflict", { id, resolution }),
  toggleWatchMode: (enabled: boolean) =>
    invoke<void>("toggle_watch_mode", { enabled }),
  pickFolder: () => invoke<string | null>("pick_folder"),
  pickUploadPaths: () => invoke<string[]>("pick_upload_paths"),
  openLogsFolder: () => invoke<void>("open_logs_folder"),
  openConfigFolder: () => invoke<void>("open_config_folder"),
  clearCliLock: () => invoke<void>("clear_cli_lock"),
};

export function formatBytesPerSec(bytes: number): string {
  if (bytes <= 0) return "—";
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(1)} MB/s`;
}

export function formatEta(seconds?: number | null): string {
  if (!seconds) return "—";
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return m > 0 ? `${m}m ${s}s` : `${s}s`;
}

export function normalizeServerUrl(input: string): string {
  let url = input.trim();
  if (!url.startsWith("http://") && !url.startsWith("https://")) {
    url = `http://${url}`;
  }
  url = url.replace(/\/+$/, "");
  if (!url.endsWith("/api")) {
    url += "/api";
  }
  return url;
}

export function webUrlFromApi(apiUrl: string): string {
  return apiUrl.replace(/\/api\/?$/, "");
}
