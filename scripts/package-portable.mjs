import fs from "fs";
import path from "path";
import { execFileSync, execSync } from "child_process";

const rootDir = process.cwd();
const isWindows = process.platform === "win32";
const isLinux = process.platform === "linux";
if (!isWindows && !isLinux) {
  console.error("Portable packaging is supported on Windows and Linux.");
  process.exit(1);
}

const pkgJson = JSON.parse(
  fs.readFileSync(path.join(rootDir, "package.json"), "utf8")
);
const version = pkgJson.version || "1.0.0";
const releaseDir = path.join(rootDir, "release");
const linuxArchiveName = `BankBake-Portable-v${version}-linux-${process.arch}`;
const releaseOutDir = path.join(releaseDir, isWindows ? `BankBake-v${version}` : linuxArchiveName);
const archivePath = isWindows
  ? path.join(releaseDir, `BankBake-Portable-v${version}-win-x64.zip`)
  : path.join(releaseDir, `${linuxArchiveName}.tar.gz`);

console.log(`=== Building BankBake v${version} Portable Release ===`);
execSync("pnpm tauri build --no-bundle", { stdio: "inherit" });

const executable = isWindows ? "bankbake.exe" : "bankbake";
const candidateExes = [
  process.env.CARGO_TARGET_DIR
    ? path.resolve(rootDir, "src-tauri", process.env.CARGO_TARGET_DIR, "release", executable)
    : "",
  path.join(rootDir, "src-tauri", "target", "release", executable),
].filter(Boolean);

const builtExe = candidateExes.find((p) => fs.existsSync(p));
if (!builtExe) {
  console.error(`Could not find compiled ${executable} in target/release.`);
  process.exit(1);
}

console.log(`Found compiled executable: ${builtExe}`);

// Clean and assemble portable folder
fs.rmSync(releaseOutDir, { recursive: true, force: true });
fs.mkdirSync(releaseOutDir, { recursive: true });

fs.copyFileSync(builtExe, path.join(releaseOutDir, isWindows ? "BankBake.exe" : executable));

// 2. Copy stripped fmodstudio/ folder
const fmodSrc = path.join(rootDir, "fmodstudio");
if ((isWindows || process.arch === "x64") && fs.existsSync(fmodSrc)) {
  console.log("Copying bundled fmodstudio/ runtime...");
  fs.cpSync(fmodSrc, path.join(releaseOutDir, "fmodstudio"), { recursive: true });
}

// 3. Copy samples/ folder
const samplesSrc = path.join(rootDir, "samples");
if (fs.existsSync(samplesSrc)) {
  console.log("Copying samples/...");
  fs.cpSync(samplesSrc, path.join(releaseOutDir, "samples"), { recursive: true });
}

// 4. Create zip archive on Windows using PowerShell Compress-Archive
if (isWindows) {
  fs.rmSync(archivePath, { force: true });
  execSync(
    `powershell -NoProfile -Command "Compress-Archive -Path '${releaseOutDir}\\*' -DestinationPath '${archivePath}' -Force"`,
    { stdio: "inherit" }
  );
} else {
  execFileSync("tar", ["-czf", archivePath, "-C", releaseDir, linuxArchiveName], {
    stdio: "inherit",
  });
}

console.log("\n=== Portable Build Complete! ===");
console.log(`Portable folder: ${releaseOutDir}`);
console.log(`Portable archive: ${archivePath}`);
