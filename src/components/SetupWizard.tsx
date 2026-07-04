import { useEffect, useState } from "react";
import { api } from "../api";
import type { AppConfig } from "../types";

interface Props {
  onComplete: () => void;
}

export function SetupWizard({ onComplete }: Props) {
  const [step, setStep] = useState(1);
  const [serverUrl, setServerUrl] = useState("https://");
  const [apiKey, setApiKey] = useState("");
  const [cliPath, setCliPath] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [serverInfo, setServerInfo] = useState("");

  useEffect(() => {
    api.detectCli().then(setCliPath).catch(() => setCliPath("Not detected"));
  }, []);

  const handleConnect = async () => {
    setError("");
    setLoading(true);
    try {
      let url = serverUrl.trim();
      if (!url.endsWith("/api")) {
        url = url.replace(/\/+$/, "") + "/api";
      }
      const info = await api.completeSetup(url, apiKey.trim());
      setServerInfo(info.version || info.raw_output || "Connected");
      setStep(3);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleFinish = async () => {
    const config = await api.getConfig();
    const updated: AppConfig = {
      ...config,
      setup_complete: true,
      cli_path: cliPath.startsWith("Not") ? null : cliPath.split(" ")[0],
    };
    await api.saveConfig(updated);
    onComplete();
  };

  return (
    <div className="setup-wizard">
      <div className="setup-card">
        <h1>Welcome to Immich Desktop</h1>
        <p className="subtitle">
          Connect to your Immich server to start syncing photos and videos.
        </p>

        <div className="steps">
          <span className={step >= 1 ? "active" : ""}>1. Server</span>
          <span className={step >= 2 ? "active" : ""}>2. API Key</span>
          <span className={step >= 3 ? "active" : ""}>3. Done</span>
        </div>

        {step === 1 && (
          <div className="step-content">
            <label>
              Immich Server URL
              <input
                type="url"
                value={serverUrl}
                onChange={(e) => setServerUrl(e.target.value)}
                placeholder="https://photos.example.com"
              />
            </label>
            <p className="hint">
              Use your server base URL. The app will append <code>/api</code> automatically.
            </p>
            <div className="cli-status">
              <strong>CLI detected:</strong> {cliPath}
            </div>
            <p className="hint">
              Requires <code>@immich/cli</code> (npm) or the{" "}
              <a
                href="https://github.com/immich-app/immich/pkgs/container/immich-cli"
                target="_blank"
                rel="noreferrer"
              >
                Docker CLI
              </a>{" "}
              via Node.js on PATH.
            </p>
            <button
              className="btn primary"
              disabled={!serverUrl.trim()}
              onClick={() => setStep(2)}
            >
              Continue
            </button>
          </div>
        )}

        {step === 2 && (
          <div className="step-content">
            <label>
              API Key
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="Paste your Immich API key"
              />
            </label>
            <p className="hint">
              Generate an API key in Immich: <strong>Account Settings → API Keys</strong>.
              Your key is stored in Windows Credential Manager, never in this repository.
            </p>
            {error && <div className="error">{error}</div>}
            <div className="btn-row">
              <button className="btn" onClick={() => setStep(1)}>
                Back
              </button>
              <button
                className="btn primary"
                disabled={!apiKey.trim() || loading}
                onClick={handleConnect}
              >
                {loading ? "Connecting…" : "Connect & Test"}
              </button>
            </div>
          </div>
        )}

        {step === 3 && (
          <div className="step-content">
            <div className="success-icon">✓</div>
            <h2>Connected successfully</h2>
            <pre className="server-info">{serverInfo}</pre>
            <button className="btn primary" onClick={handleFinish}>
              Open Dashboard
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
