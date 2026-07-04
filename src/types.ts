export interface HandshakeResult {
  success: boolean;
  status_code: number;
  server_version?: string | null;
  message: string;
}

export interface DiscoveredServer {
  name: string;
  url: string;
  source: string;
}

export interface UploadOptions {
  recursive: boolean;
  concurrency: number;
  album: boolean;
  album_name?: string | null;
  ignore_patterns: string[];
  include_hidden: boolean;
  dry_run: boolean;
  skip_hash: boolean;
}

export interface WatchModeConfig {
  enabled: boolean;
  debounce_ms: number;
}

export interface ScheduleConfig {
  enabled: boolean;
  start_hour: number;
  end_hour: number;
}

export interface SyncTriggersConfig {
  wifi_only: boolean;
  allowed_networks: string[];
  require_plugged_in: boolean;
  schedule: ScheduleConfig;
}

export interface AppConfig {
  server_url?: string | null;
  watch_folders: string[];
  upload_options: UploadOptions;
  watch_mode: WatchModeConfig;
  sync_triggers: SyncTriggersConfig;
  cli_path?: string | null;
  use_credential_manager: boolean;
  start_minimized: boolean;
  setup_complete: boolean;
}

export type FileStatus =
  | "queued"
  | "uploading"
  | "skipped"
  | "failed"
  | "completed";

export interface FileActivity {
  id: string;
  path: string;
  status: FileStatus;
  message?: string | null;
  timestamp: string;
}

export interface UploadProgress {
  total_files: number;
  completed_files: number;
  failed_files: number;
  skipped_files: number;
  bytes_per_second: number;
  eta_seconds?: number | null;
  is_running: boolean;
  is_paused: boolean;
  current_file?: string | null;
}

export interface SyncTriggerStatus {
  can_sync: boolean;
  wifi_connected: boolean;
  on_allowed_network: boolean;
  plugged_in: boolean;
  within_schedule: boolean;
  reasons: string[];
}

export interface RetryItem {
  id: string;
  path: string;
  error: string;
  attempts: number;
  last_attempt: string;
}

export interface RetryQueue {
  items: RetryItem[];
  max_attempts: number;
}

export interface ConflictItem {
  id: string;
  local_path: string;
  remote_info?: string | null;
  local_modified?: string | null;
  resolution?: string | null;
}

export interface ConflictStore {
  conflicts: ConflictItem[];
}

export interface ServerInfo {
  version: string;
  raw_output: string;
}

export const defaultConfig = (): AppConfig => ({
  server_url: null,
  watch_folders: [],
  upload_options: {
    recursive: true,
    concurrency: 4,
    album: false,
    album_name: null,
    ignore_patterns: [],
    include_hidden: false,
    dry_run: false,
    skip_hash: false,
  },
  watch_mode: { enabled: false, debounce_ms: 5000 },
  sync_triggers: {
    wifi_only: false,
    allowed_networks: [],
    require_plugged_in: false,
    schedule: { enabled: false, start_hour: 22, end_hour: 6 },
  },
  cli_path: null,
  use_credential_manager: true,
  start_minimized: false,
  setup_complete: false,
});
