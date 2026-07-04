import { existsSync, copyFileSync, readdirSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const version = pkg.version;

console.log(`Preflight build for Immich Desktop v${version}`);

execSync("npm run build:check", { cwd: root, stdio: "inherit" });
execSync("npm run build", { cwd: root, stdio: "inherit" });
execSync("npm run tauri build", { cwd: root, stdio: "inherit" });

const releaseDir = join(root, "src-tauri", "target", "release");
const bundleDir = join(releaseDir, "bundle", "nsis");
const distDir = join(root, "dist", "release");

const candidates = [
  join(releaseDir, "immich-desktop.exe"),
  join(releaseDir, "ImmichDesktop.exe"),
  join(releaseDir, "Immich Desktop.exe"),
];

if (existsSync(bundleDir)) {
  for (const file of readdirSync(bundleDir)) {
    if (file.endsWith(".exe")) {
      candidates.push(join(bundleDir, file));
    }
  }
}

const source = candidates.find((p) => existsSync(p));
if (!source) {
  console.error("Build failed: no executable found in target/release");
  process.exit(1);
}

import { mkdirSync } from "node:fs";
mkdirSync(distDir, { recursive: true });

const portableName = `ImmichDesktop-v${version}.exe`;
const portablePath = join(distDir, portableName);
copyFileSync(source, portablePath);

console.log(`Verified executable: ${portablePath}`);
console.log("Preflight build passed.");
