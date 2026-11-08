import { createRequire } from "node:module";
import { existsSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  createRenderer,
  enableAutomation,
  render,
  startFrameLoop,
} from "@gpuix/react";
import { ProcessBackend, resourcePath } from "./backend";
import { AppModel } from "./model";
import { App } from "./ui";
import type { Desktop } from "./desktop";

let desktop: Desktop | null = null;
if (process.platform === "win32") {
  const installed = resourcePath("ssh-router-desktop.node");
  const path = existsSync(installed)
    ? installed
    : resolve(import.meta.dir, "../dist/resources/ssh-router-desktop.node");
  desktop = createRequire(import.meta.url)(path) as Desktop;
  if (!desktop.acquireInstance(process.env.SSH_ROUTER_INSTANCE_NAME))
    process.exit(0);
}

const backend = new ProcessBackend();
const model = new AppModel(backend);
process.on("exit", () => model.close());
const renderer = createRenderer();
renderer.init({
  title: "SSH Router",
  appName: "SSH Router",
  appId: "sshrouter.SSHRouter",
  titlebarTransparent: process.platform === "win32",
  width: Number(process.env.SSH_ROUTER_WIDTH) || 620,
  height: Number(process.env.SSH_ROUTER_HEIGHT) || 760,
  minWidth: 520,
  minHeight: 440,
  show:
    !desktop ||
    process.env.SSH_ROUTER_SHOW === "1" ||
    !!process.env.SSH_ROUTER_STARTUP_MARKER,
  focus: process.env.GPUIX_BACKGROUND !== "1",
});
render(
  <App
    model={model}
    desktop={desktop}
    onReady={() => {
      if (process.env.SSH_ROUTER_STARTUP_MARKER)
        writeFileSync(
          process.env.SSH_ROUTER_STARTUP_MARKER,
          String(Date.now()),
        );
    }}
  />,
  { renderer, keyboardFocusDim: false },
);
enableAutomation(renderer);
startFrameLoop(renderer, { onTerminated: () => process.exit(0) });

if (desktop) {
  let attempts = 0;
  const timer = setInterval(() => {
    try {
      if (desktop!.attachWindow()) {
        clearInterval(timer);
        return;
      }
      if (++attempts < 30) return;
      throw new Error("无法安装托盘，关闭窗口将退出应用");
    } catch (error) {
      clearInterval(timer);
      renderer.activateWindow();
      model.notice(
        error instanceof Error ? error.message : String(error),
        true,
      );
    }
  }, 100);
}
