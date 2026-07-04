import { existsSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const errors = [];

function check(label, ok, detail) {
  if (!ok) errors.push(`${label}: ${detail}`);
  else console.log(`  OK  ${label}`);
}

console.log("Build environment check");

check("Node.js", !!process.version, process.version);
try {
  const rustc = execSync("rustc --version", { encoding: "utf8" }).trim();
  check("Rust", true, rustc);
} catch {
  check("Rust", false, "rustc not found");
}
try {
  const cargo = execSync("cargo --version", { encoding: "utf8" }).trim();
  check("Cargo", true, cargo);
} catch {
  check("Cargo", false, "cargo not found");
}

const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const tauriConf = JSON.parse(
  readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"),
);
const cargoToml = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");

check("package.json version", !!pkg.version, pkg.version);
check("tauri.conf.json version", tauriConf.version === pkg.version,
  `mismatch (${tauriConf.version} vs ${pkg.version})`);
check("Cargo.toml version", cargoToml.includes(`version = "${pkg.version}"`),
  `expected ${pkg.version}`);

check("node_modules", existsSync(join(root, "node_modules")), "run npm install");
check("src-tauri/icons", existsSync(join(root, "src-tauri", "icons")), "missing icons");

if (errors.length) {
  console.error("\nBuild check failed:");
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}

console.log("\nBuild check passed.");
