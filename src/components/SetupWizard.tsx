import { useCallback, useEffect, useState } from "react";
import { api, normalizeServerUrl, webUrlFromApi } from "../api";
import type { DiscoveredServer } from "../types";

interface Props {
  onComplete: () => void;
}

export function SetupWizard({ onComplete }: Props) {
  const [step, setStep] = useState(1);
  const [discovered, setDiscovered] = useState<DiscoveredServer[]>([]);
  const [scanning, setScanning] = useState(false);
  const [selectedServer, setSelectedServer] = useState("");
  const [manualUrl, setManualUrl] = useState("");
  const [useManual, setUseManual] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [cliPath, setCliPath] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [handshakeStatus, setHandshakeStatus] = useState("");
  const [serverInfo, setServerInfo] = useState("");

  const resolvedUrl = useManual
    ? manualUrl.trim()
    : selectedServer || manualUrl.trim();

  const scanNetwork = useCallback(async () => {
    setScanning(true);
    setError("");
    try {
      const servers = await api.discoverServers();
      setDiscovered(servers);
      if (servers.length === 1) {
        setSelectedServer(servers[0].url);
        setUseManual(false);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => {
    api.detectCli().then(setCliPath).catch(() => setCliPath("Not detected"));
    scanNetwork();
  }, [scanNetwork]);

  const handleHandshake = async () => {
    setError("");
    setHandshakeStatus("");
    setLoading(true);
    try {
      const url = normalizeServerUrl(resolvedUrl);
      const result = await api.performHandshake(url, apiKey.trim());

      if (!result.success || (result.status_code !== 200 && result.status_code !== 204)) {
        setError(
          `Handshake failed (HTTP ${result.status_code}): ${result.message}`,
        );
        return;
      }

      setHandshakeStatus(
        `Handshake OK (HTTP ${result.status_code})${
          result.server_version ? ` — v${result.server_version}` : ""
        }`,
      );
      setServerInfo(result.server_version ?? "Server reachable");
      setStep(4);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleFinish = async () => {
    setLoading(true);
    setError("");
    try {
      const url = normalizeServerUrl(resolvedUrl);
      const info = await api.finalizeSetup(url, apiKey.trim());
      setServerInfo(info.version || info.raw_output || serverInfo);
      const config = await api.getConfig();
      await api.saveConfig({
        ...config,
        setup_complete: true,
        server_url: url,
        cli_path: cliPath.startsWith("Not") ? null : cliPath.split(" ")[0],
      });
      onComplete();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="setup-wizard">
      <div className="setup-card setup-card-wide">
        <h1>Welcome to Immich Desktop</h1>
        <p className="subtitle">
          Connect to your Immich server — auto-discover on your network or enter
          a URL manually.
        </p>

        <div className="steps">
          <span className={step >= 1 ? "active" : ""}>1. Server</span>
          <span className={step >= 2 ? "active" : ""}>2. API Key</span>
          <span className={step >= 3 ? "active" : ""}>3. Handshake</span>
          <span className={step >= 4 ? "active" : ""}>4. Done</span>
        </div>

        {step === 1 && (
          <div className="step-content">
            <div className="discovery-header">
              <h2>Auto-Discovery</h2>
              <button
                className="btn"
                onClick={scanNetwork}
                disabled={scanning}
              >
                {scanning ? "Scanning…" : "Scan Network"}
              </button>
            </div>

            {discovered.length > 0 ? (
              <label>
                Discovered servers
                <select
                  value={useManual ? "" : selectedServer}
                  onChange={(e) => {
                    setUseManual(false);
                    setSelectedServer(e.target.value);
                  }}
                >
                  <option value="">Select a server…</option>
                  {discovered.map((s) => (
                    <option key={s.url} value={s.url}>
                      {s.name} ({s.source}) — {webUrlFromApi(s.url)}
                    </option>
                  ))}
                </select>
              </label>
            ) : (
              <p className="hint">
                {scanning
                  ? "Searching local network via mDNS and subnet scan…"
                  : "No servers found on the local network. Use manual entry below."}
              </p>
            )}

            <label className="checkbox">
              <input
                type="checkbox"
                checked={useManual}
                onChange={(e) => setUseManual(e.target.checked)}
              />
              Enter server URL manually (remote, proxy, or different subnet)
            </label>

            {(useManual || discovered.length === 0) && (
              <label>
                Server URL
                <input
                  type="url"
                  value={manualUrl}
                  onChange={(e) => setManualUrl(e.target.value)}
                  placeholder="https://photos.example.com or http://192.168.1.10:2283"
                />
              </label>
            )}

            <div className="cli-status">
              <strong>CLI detected:</strong> {cliPath}
            </div>

            {error && <div className="error">{error}</div>}

            <button
              className="btn primary"
              disabled={!resolvedUrl.trim()}
              onClick={() => setStep(2)}
            >
              Continue
            </button>
          </div>
        )}

        {step === 2 && (
          <div className="step-content">
            <p className="hint">
              Server: <code>{normalizeServerUrl(resolvedUrl)}</code>
            </p>
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
              Generate in Immich: <strong>Account Settings → API Keys</strong>.
              Stored in Windows Credential Manager — never in this repository.
            </p>
            <div className="btn-row">
              <button className="btn" onClick={() => setStep(1)}>
                Back
              </button>
              <button
                className="btn primary"
                disabled={!apiKey.trim()}
                onClick={() => setStep(3)}
              >
                Continue to Handshake
              </button>
            </div>
          </div>
        )}

        {step === 3 && (
          <div className="step-content">
            <p className="hint">
              The app will verify your server responds with HTTP 200 before
              proceeding. No CLI processes are started during the handshake.
            </p>
            {handshakeStatus && (
              <div className="alert success">{handshakeStatus}</div>
            )}
            {error && <div className="error">{error}</div>}
            <div className="btn-row">
              <button className="btn" onClick={() => setStep(2)}>
                Back
              </button>
              <button
                className="btn primary"
                disabled={!apiKey.trim() || loading}
                onClick={handleHandshake}
              >
                {loading ? "Testing connection…" : "Run Handshake Test"}
              </button>
            </div>
          </div>
        )}

        {step === 4 && (
          <div className="step-content">
            <div className="success-icon">✓</div>
            <h2>Connected successfully</h2>
            <pre className="server-info">{serverInfo}</pre>
            <button className="btn primary" onClick={handleFinish} disabled={loading}>
              {loading ? "Finalizing…" : "Open Dashboard"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
