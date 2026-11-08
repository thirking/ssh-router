import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { launch } from "@gpuix/react/automation";
import { expect } from "bun:test";

const temporary = mkdtempSync(join(tmpdir(), "ssh-router-gpuix-"));
const config = join(temporary, "config.json");
const output = process.env.SSH_ROUTER_SCREENSHOT_DIR || temporary;
const app = await launch({
  command: process.env.SSH_ROUTER_APP_EXECUTABLE || "bun",
  args: process.env.SSH_ROUTER_APP_EXECUTABLE ? [] : ["src/main.tsx"],
  cwd: resolve(import.meta.dir, ".."),
  env: {
    ...process.env,
    GPUIX_BACKGROUND: "1",
    SSH_ROUTER_SHOW: "1",
    SSH_ROUTER_CONFIG: config,
    SSH_ROUTER_INSTANCE_NAME: `Local\\SSHRouterSmoke-${process.pid}`,
  },
});
async function waitUntil(check: () => Promise<boolean>, message: string) {
  const deadline = Date.now() + 5000;
  while (!(await check())) {
    if (Date.now() > deadline) throw new Error(message);
    await Bun.sleep(30);
  }
}
async function settle() {
  await app.clock.fastForward(300);
}
async function reveal(testId: string) {
  const container = await app.getByTestId("content").element();
  const bounds = await app.getByTestId(testId).bounds();
  const viewport = container.bounds!;
  if (
    bounds.y < viewport.y ||
    bounds.y + bounds.height > viewport.y + viewport.height
  ) {
    const { offset } = await app.call("getScrollOffset", {
      elementId: container.id,
    });
    await app.call("scrollTo", {
      elementId: container.id,
      x: 0,
      y:
        (offset?.[1] ?? 0) +
        viewport.y +
        viewport.height / 2 -
        bounds.y -
        bounds.height / 2,
    });
    await settle();
  }
}
try {
  await app.getByTestId("create-config").waitFor({ timeoutMs: 60_000 });
  console.log("创建默认配置");
  await app.getByTestId("create-config").click();
  await app.getByTestId("edit-22").waitFor();
  console.log("默认配置已显示");
  await app.getByTestId("search").fill("Git Bash");
  expect(await app.getByTestId("edit-2222").count()).toBe(1);
  expect(await app.getByTestId("edit-22").count()).toBe(0);
  await app.getByTestId("search").fill("");
  await settle();
  await app.screenshot({ path: join(output, "gpuix-main.png") });
  console.log("搜索验证通过");
  await reveal("edit-2222");
  await app.getByTestId("edit-2222").click();
  await app.getByTestId("route-dialog").waitFor();
  await app.getByTestId("route-port").fill("22");
  console.log("提交表单");
  await app.getByTestId("apply-route").click();
  expect(await app.getByText("此端口已有路由").count()).toBeGreaterThan(0);
  await app.getByTestId("route-port").fill("2222");
  await app.getByTestId("route-name").fill("中文路由测试");
  const dialogBounds = await app.getByTestId("route-dialog").bounds();
  const nameBounds = await app.getByTestId("route-name").bounds();
  expect(nameBounds.x + nameBounds.width).toBeLessThanOrEqual(
    dialogBounds.x + dialogBounds.width - 12,
  );
  await settle();
  await app.screenshot({ path: join(output, "gpuix-editor.png") });
  console.log("提交表单");
  await app.getByTestId("apply-route").click();
  await waitUntil(
    async () => (await app.getByTestId("route-dialog").count()) === 0,
    "编辑弹窗未关闭",
  );
  await app.getByText("中文路由测试").waitFor();
  await app.getByTestId("reload").click();
  await app.getByTestId("confirm-dialog").waitFor();
  await app.getByTestId("confirm-cancel").click();
  await waitUntil(
    async () => (await app.getByTestId("confirm-dialog").count()) === 0,
    "确认弹窗未关闭",
  );
  await app.getByTestId("save").click();
  const deadline = Date.now() + 5000;
  while (!readFileSync(config, "utf8").includes("中文路由测试")) {
    if (Date.now() > deadline) throw new Error("配置未保存");
    await Bun.sleep(50);
  }
  await reveal("delete-2222");
  await app.getByTestId("delete-2222").click();
  await app.getByTestId("confirm-cancel").click();
  expect(await app.getByTestId("edit-2222").count()).toBe(1);
  await app.getByTestId("tab-system").click();
  await app.getByText("安装状态").waitFor();
  await settle();
  await app.screenshot({ path: join(output, "gpuix-system.png") });
  await app.getByTestId("tab-routes").click();
  await app.getByTestId("save").press("ctrl-n");
  await app.getByTestId("route-dialog").waitFor();
  await app.getByTestId("route-port").press("escape");
  await waitUntil(
    async () => (await app.getByTestId("route-dialog").count()) === 0,
    "Escape 未关闭弹窗",
  );
  await app.getByTestId("save").press("ctrl-n");
  await app.getByTestId("route-dialog").waitFor();
  await app.getByTestId("route-port").fill("3333");
  await app.getByTestId("route-name").fill("快捷键新增");
  await app.getByTestId("route-shell").fill("bash.exe");
  await app.getByTestId("apply-route").click();
  await waitUntil(
    async () => (await app.getByTestId("route-dialog").count()) === 0,
    "新增路由未应用",
  );
  await app.getByTestId("reload").press("ctrl-s");
  await waitUntil(
    async () => readFileSync(config, "utf8").includes("3333"),
    "Ctrl+S 未保存配置",
  );
  console.log(`gpuix 交互验证通过；截图目录：${output}`);
} catch (error) {
  await app.screenshot({ path: join(output, "gpuix-error.png") });
  console.log(JSON.stringify(await app.getByTestId("app").element()));
  throw error;
} finally {
  await app.close();
  rmSync(temporary, { recursive: true, force: true });
}
