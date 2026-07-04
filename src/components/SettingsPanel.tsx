import { useState } from "react";
import { api } from "../api";
import type { AppConfig, ConflictItem, RetryQueue } from "../types";

interface Props {
  config: AppConfig;
  retryQueue: RetryQueue;
  conflicts: ConflictItem[];
  onSave: (config: AppConfig) => void;
  onRefresh: () => void;
}

export function SettingsPanel({
  config,
  retryQueue,
  conflicts,
  onSave,
  onRefresh,
}: Props) {
  const [draft, setDraft] = useState<AppConfig>(config);
  const [networkInput, setNetworkInput] = useState("");
  const [ignoreInput, setIgnoreInput] = useState("");
  const [saved, setSaved] = useState(false);

  const update = (patch: Partial<AppConfig>) => {
    setDraft((d) => ({ ...d, ...patch }));
    setSaved(false);
  };

  const handleSave = async () => {
    await api.saveConfig(draft);
    if (draft.watch_mode.enabled !== config.watch_mode.enabled) {
      await api.toggleWatchMode(draft.watch_mode.enabled);
    }
    onSave(draft);
    setSaved(true);
  };

  const addWatchFolder = async () => {
    const folder = await api.pickFolder();
    if (folder && !draft.watch_folders.includes(folder)) {
      update({ watch_folders: [...draft.watch_folders, folder] });
    }
  };

  const addNetwork = () => {
    const name = networkInput.trim();
    if (name && !draft.sync_triggers.allowed_networks.includes(name)) {
      update({
        sync_triggers: {
          ...draft.sync_triggers,
          allowed_networks: [...draft.sync_triggers.allowed_networks, name],
        },
      });
      setNetworkInput("");
    }
  };

  const addIgnore = () => {
    const pattern = ignoreInput.trim();
    if (pattern && !draft.upload_options.ignore_patterns.includes(pattern)) {
      update({
        upload_options: {
          ...draft.upload_options,
          ignore_patterns: [...draft.upload_options.ignore_patterns, pattern],
        },
      });
      setIgnoreInput("");
    }
  };

  return (
    <div className="settings">
      <header className="page-header">
        <div>
          <h1>Settings</h1>
          <p>Configure sync behavior, watch mode, and upload options</p>
        </div>
        <div className="header-actions">
          <button className="btn" onClick={() => api.openConfigFolder()}>
            Open Config Folder
          </button>
          <button className="btn primary" onClick={handleSave}>
            Save Settings
          </button>
        </div>
      </header>

      {saved && <div className="alert success">Settings saved.</div>}

      <section className="settings-section">
        <h2>Connection</h2>
        <label>
          Server URL
          <input
            type="url"
            value={draft.server_url ?? ""}
            onChange={(e) => update({ server_url: e.target.value })}
          />
        </label>
        <label>
          CLI Path (optional)
          <input
            type="text"
            value={draft.cli_path ?? ""}
            onChange={(e) => update({ cli_path: e.target.value || null })}
            placeholder="immich or full path to immich.exe"
          />
        </label>
      </section>

      <section className="settings-section">
        <h2>Watch Mode</h2>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={draft.watch_mode.enabled}
            onChange={(e) =>
              update({
                watch_mode: { ...draft.watch_mode, enabled: e.target.checked },
              })
            }
          />
          Enable background folder monitoring
        </label>
        <label>
          Debounce (ms)
          <input
            type="number"
            min={1000}
            step={1000}
            value={draft.watch_mode.debounce_ms}
            onChange={(e) =>
              update({
                watch_mode: {
                  ...draft.watch_mode,
                  debounce_ms: Number(e.target.value),
                },
              })
            }
          />
        </label>
        <div className="folder-list">
          {draft.watch_folders.map((f) => (
            <div key={f} className="folder-row">
              <code>{f}</code>
              <button
                className="btn small"
                onClick={() =>
                  update({
                    watch_folders: draft.watch_folders.filter((x) => x !== f),
                  })
                }
              >
                Remove
              </button>
            </div>
          ))}
        </div>
        <button className="btn" onClick={addWatchFolder}>
          Add Watch Folder
        </button>
      </section>

      <section className="settings-section">
        <h2>Sync Triggers</h2>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={draft.sync_triggers.wifi_only}
            onChange={(e) =>
              update({
                sync_triggers: {
                  ...draft.sync_triggers,
                  wifi_only: e.target.checked,
                },
              })
            }
          />
          Wi-Fi only
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={draft.sync_triggers.require_plugged_in}
            onChange={(e) =>
              update({
                sync_triggers: {
                  ...draft.sync_triggers,
                  require_plugged_in: e.target.checked,
                },
              })
            }
          />
          Require device plugged in
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={draft.sync_triggers.schedule.enabled}
            onChange={(e) =>
              update({
                sync_triggers: {
                  ...draft.sync_triggers,
                  schedule: {
                    ...draft.sync_triggers.schedule,
                    enabled: e.target.checked,
                  },
                },
              })
            }
          />
          Limit to time window
        </label>
        {draft.sync_triggers.schedule.enabled && (
          <div className="inline-fields">
            <label>
              Start hour
              <input
                type="number"
                min={0}
                max={23}
                value={draft.sync_triggers.schedule.start_hour}
                onChange={(e) =>
                  update({
                    sync_triggers: {
                      ...draft.sync_triggers,
                      schedule: {
                        ...draft.sync_triggers.schedule,
                        start_hour: Number(e.target.value),
                      },
                    },
                  })
                }
              />
            </label>
            <label>
              End hour
              <input
                type="number"
                min={0}
                max={23}
                value={draft.sync_triggers.schedule.end_hour}
                onChange={(e) =>
                  update({
                    sync_triggers: {
                      ...draft.sync_triggers,
                      schedule: {
                        ...draft.sync_triggers.schedule,
                        end_hour: Number(e.target.value),
                      },
                    },
                  })
                }
              />
            </label>
          </div>
        )}
        <div className="inline-fields">
          <input
            type="text"
            value={networkInput}
            onChange={(e) => setNetworkInput(e.target.value)}
            placeholder="Allowed Wi-Fi network name"
          />
          <button className="btn" onClick={addNetwork}>
            Add Network
          </button>
        </div>
        <div className="tag-list">
          {draft.sync_triggers.allowed_networks.map((n) => (
            <span key={n} className="tag">
              {n}
              <button
                onClick={() =>
                  update({
                    sync_triggers: {
                      ...draft.sync_triggers,
                      allowed_networks:
                        draft.sync_triggers.allowed_networks.filter(
                          (x) => x !== n,
                        ),
                    },
                  })
                }
              >
                ×
              </button>
            </span>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h2>Upload Options</h2>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={draft.upload_options.recursive}
            onChange={(e) =>
              update({
                upload_options: {
                  ...draft.upload_options,
                  recursive: e.target.checked,
                },
              })
            }
          />
          Recursive
        </label>
        <label>
          Concurrency
          <input
            type="number"
            min={1}
            max={16}
            value={draft.upload_options.concurrency}
            onChange={(e) =>
              update({
                upload_options: {
                  ...draft.upload_options,
                  concurrency: Number(e.target.value),
                },
              })
            }
          />
        </label>
        <div className="inline-fields">
          <input
            type="text"
            value={ignoreInput}
            onChange={(e) => setIgnoreInput(e.target.value)}
            placeholder="Ignore glob pattern"
          />
          <button className="btn" onClick={addIgnore}>
            Add Pattern
          </button>
        </div>
      </section>

      <section className="settings-section">
        <h2>Retry Queue ({retryQueue.items.length})</h2>
        {retryQueue.items.length === 0 ? (
          <p className="empty">No failed uploads in the retry queue.</p>
        ) : (
          retryQueue.items.map((item) => (
            <div key={item.id} className="retry-row">
              <code>{item.path}</code>
              <span>{item.attempts}/{retryQueue.max_attempts} attempts</span>
              <button
                className="btn small"
                onClick={async () => {
                  await api.removeFromRetryQueue(item.id);
                  onRefresh();
                }}
              >
                Remove
              </button>
            </div>
          ))
        )}
      </section>

      <section className="settings-section">
        <h2>Conflicts ({conflicts.length})</h2>
        {conflicts.length === 0 ? (
          <p className="empty">No unresolved conflicts.</p>
        ) : (
          conflicts.map((c) => (
            <ConflictRow key={c.id} conflict={c} onRefresh={onRefresh} />
          ))
        )}
      </section>
    </div>
  );
}

function ConflictRow({
  conflict,
  onRefresh,
}: {
  conflict: ConflictItem;
  onRefresh: () => void;
}) {
  return (
    <div className="conflict-row">
      <div className="conflict-side">
        <strong>Local</strong>
        <code>{conflict.local_path}</code>
        {conflict.local_modified && <span>{conflict.local_modified}</span>}
      </div>
      <div className="conflict-side">
        <strong>Remote</strong>
        <span>{conflict.remote_info ?? "Unknown"}</span>
      </div>
      <div className="conflict-actions">
        <button
          className="btn small"
          onClick={async () => {
            await api.resolveConflict(conflict.id, "keep_local");
            onRefresh();
          }}
        >
          Keep Local
        </button>
        <button
          className="btn small"
          onClick={async () => {
            await api.resolveConflict(conflict.id, "keep_remote");
            onRefresh();
          }}
        >
          Keep Remote
        </button>
        <button
          className="btn small primary"
          onClick={async () => {
            await api.resolveConflict(conflict.id, "upload_both");
            onRefresh();
          }}
        >
          Upload Both
        </button>
      </div>
    </div>
  );
}
