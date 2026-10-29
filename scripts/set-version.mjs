// Sets the app version everywhere it lives: Cargo.toml ([workspace.package]),
// src-tauri/tauri.conf.json and package.json (and their lock files).
// Usage: npm run version:set -- 0.2.0
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2]?.replace(/^v/, "");
if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(version)) {
  console.error("Usage: npm run version:set -- <major.minor.patch>, e.g. 0.2.0");
  process.exit(1);
}

function edit(file, from, to) {
  const text = readFileSync(file, "utf8");
  if (!from.test(text)) throw new Error(`${file}: no version found`);
  writeFileSync(file, text.replace(from, to));
}

edit("Cargo.toml", /(\[workspace\.package\][^[]*?\nversion = ")[^"]+(")/, `$1${version}$2`);
edit("src-tauri/tauri.conf.json", /("version": ")[^"]+(")/, `$1${version}$2`);
// npm also updates package-lock.json; no git tag (the release workflow is tag-driven).
execSync(`npm version ${version} --no-git-tag-version --allow-same-version`, { stdio: "ignore" });
// Refresh Cargo.lock's entries for the workspace crates.
execSync("cargo update --workspace --offline", { stdio: "inherit" });
console.log(`Version set to ${version}. Add it to CHANGELOG.md, commit, then tag v${version}.`);
