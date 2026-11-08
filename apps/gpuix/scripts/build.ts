import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { createRequire } from "node:module";

const root = resolve(import.meta.dir, "../../..");
const app = resolve(import.meta.dir, "..");
const version: string = JSON.parse(
  readFileSync(resolve(app, "package.json"), "utf8"),
).version;
const windows = process.platform === "win32";
if (windows && process.arch !== "x64")
  throw new Error("Windows 安装包需要 x64 构建环境");
const target = windows ? "x86_64-pc-windows-msvc" : null;
const command = [
  "cargo",
  "build",
  "-p",
  "ssh-router-gpuix",
  "-p",
  "ssh-router-cli",
  "--release",
];
if (target) command.push("--target", target);
const build = Bun.spawnSync(command, {
  cwd: root,
  stdout: "inherit",
  stderr: "inherit",
});
if (build.exitCode) process.exit(build.exitCode);
const output = resolve(app, "dist");
const resources = resolve(output, "resources");
mkdirSync(resources, { recursive: true });
const platformPackage = windows
  ? "@gpuix/native-win32-x64-msvc"
  : process.platform === "darwin" && process.arch === "arm64"
    ? "@gpuix/native-darwin-arm64"
    : process.platform === "linux" && process.arch === "x64"
      ? "@gpuix/native-linux-x64-gnu"
      : null;
if (!platformPackage)
  throw new Error(`不支持的平台: ${process.platform}/${process.arch}`);
copyFileSync(
  createRequire(import.meta.url).resolve(platformPackage),
  resolve(resources, "gpuix-native.node"),
);
const release = resolve(root, "target", ...(target ? [target] : []), "release");
const exe = windows ? ".exe" : "";
copyFileSync(
  resolve(release, `ssh-router-backend${exe}`),
  resolve(output, `ssh-router-backend${exe}`),
);
copyFileSync(
  resolve(release, `ssh-router-cli${exe}`),
  resolve(resources, `ssh-router-cli${exe}`),
);
if (windows)
  copyFileSync(
    resolve(release, "ssh_router_desktop.dll"),
    resolve(resources, "ssh-router-desktop.node"),
  );
const args = [
  "bun",
  "build",
  "--compile",
  "--no-compile-autoload-dotenv",
  "src/bootstrap.ts",
  "--outfile",
  `dist/ssh-router${exe}`,
];
if (windows)
  args.push(
    "--windows-hide-console",
    "--windows-icon",
    resolve(root, "installer/icon.ico"),
    "--windows-title",
    "SSH Router",
    "--windows-publisher",
    "sshrouter",
    "--windows-version",
    `${version}.0`,
    "--windows-description",
    "SSH Router",
  );
const bundle = Bun.spawnSync(args, {
  cwd: app,
  stdout: "inherit",
  stderr: "inherit",
});
process.exit(bundle.exitCode);
