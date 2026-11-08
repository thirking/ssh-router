import { copyFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../../..");
const build = Bun.spawnSync(["cargo", "build", "-p", "ssh-router-gpuix"], {
  cwd: root,
  stdout: "inherit",
  stderr: "inherit",
});
if (build.exitCode) process.exit(build.exitCode);
if (process.platform === "win32") {
  const resources = resolve(import.meta.dir, "../dist/resources");
  mkdirSync(resources, { recursive: true });
  copyFileSync(
    resolve(root, "target/debug/ssh_router_desktop.dll"),
    resolve(resources, "ssh-router-desktop.node"),
  );
}
const app = Bun.spawn(["bun", "src/main.tsx"], {
  cwd: resolve(import.meta.dir, ".."),
  env: { ...process.env, SSH_ROUTER_SHOW: "1" },
  stdout: "inherit",
  stderr: "inherit",
  stdin: "inherit",
});
process.exit(await app.exited);
