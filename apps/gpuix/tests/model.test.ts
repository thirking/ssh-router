import { describe, expect, test } from "bun:test";
import { BackendError, type Backend, type Config } from "../src/backend";
import { AppModel, dirty, filterRoutes, routeErrors } from "../src/model";

const config: Config = {
  routes: [
    {
      port: 22,
      name: "PowerShell",
      shell: "pwsh.exe",
      interactiveTemplate: '"{shell}" -l',
      commandTemplate: '"{shell}" -File "{tmpfile}"',
      tmpFileExt: ".ps1",
      default: true,
    },
    {
      port: 2222,
      name: "Git Bash",
      shell: "bash.exe",
      interactiveTemplate: '"{shell}" -l',
      commandTemplate: '"{shell}" "{tmpfile}"',
      tmpFileExt: ".sh",
      default: false,
    },
  ],
  sftpCommand: "sftp-server.exe",
};

function setup(
  handler?: (method: string, params?: unknown) => Promise<unknown>,
) {
  const calls: string[] = [];
  const backend: Backend = {
    async request<T>(method: string, params?: unknown): Promise<T> {
      calls.push(method);
      return (await handler?.(method, params)) as T;
    },
    close() {},
  };
  const model = new AppModel(backend);
  model.patch({
    config: structuredClone(config),
    saved: structuredClone(config),
  });
  return { model, calls };
}

describe("配置草稿", () => {
  test("保存过程中产生的新修改保持未保存", async () => {
    let complete!: () => void;
    const { model } = setup(
      () =>
        new Promise<void>((resolve) => {
          complete = resolve;
        }),
    );
    model.edit({ ...config, sftpCommand: "first.exe" });
    const saving = model.save();
    model.edit({ ...config, sftpCommand: "newer.exe" });
    complete();
    await saving;
    expect(model.getSnapshot().saved?.sftpCommand).toBe("first.exe");
    expect(model.getSnapshot().config?.sftpCommand).toBe("newer.exe");
    expect(dirty(model.getSnapshot())).toBe(true);
    model.close();
  });
  test("失败后保留草稿且允许重试", async () => {
    let failed = true;
    const { model } = setup(async () => {
      if (failed) throw new Error("磁盘不可写");
    });
    model.edit({ ...config, sftpCommand: "new.exe" });
    expect(await model.save()).toBe(false);
    expect(dirty(model.getSnapshot())).toBe(true);
    expect(model.getSnapshot().busy).toBeNull();
    failed = false;
    expect(await model.save()).toBe(true);
    expect(dirty(model.getSnapshot())).toBe(false);
    model.close();
  });
  test("连续点击只发送一次保存", async () => {
    let complete!: () => void;
    const { model, calls } = setup(
      () =>
        new Promise<void>((resolve) => {
          complete = resolve;
        }),
    );
    model.edit({ ...config, sftpCommand: "new.exe" });
    const first = model.save();
    expect(await model.save()).toBe(false);
    expect(calls).toEqual(["config.save"]);
    complete();
    await first;
    model.close();
  });
  test("共享核心拒绝编辑时不污染原路由", async () => {
    const { model } = setup(async () => {
      throw new BackendError("validation", "必须有一条默认路由");
    });
    expect(
      await model.acceptRoute(0, { ...config.routes[0]!, default: false }),
    ).toBe(false);
    expect(model.getSnapshot().config).toEqual(config);
    model.close();
  });
  test("切换默认与删除默认路由均保留唯一默认项", () => {
    const { model } = setup();
    model.setDefault(1);
    expect(
      model
        .getSnapshot()
        .config?.routes.filter((r) => r.default)
        .map((r) => r.port),
    ).toEqual([2222]);
    model.removeRoute(1);
    expect(model.getSnapshot().config?.routes).toHaveLength(1);
    expect(model.getSnapshot().config?.routes[0]?.default).toBe(true);
    model.removeRoute(0);
    expect(model.getSnapshot().config?.routes).toHaveLength(1);
    model.close();
  });
  test("默认路由编辑时可转移到新路由", async () => {
    const { model } = setup(async () => undefined);
    await model.acceptRoute(null, {
      ...config.routes[1]!,
      port: 2223,
      default: true,
    });
    expect(
      model
        .getSnapshot()
        .config?.routes.filter((r) => r.default)
        .map((r) => r.port),
    ).toEqual([2223]);
    model.close();
  });
  test("取消 UAC 提示为中性反馈且忙碌状态清除", async () => {
    const { model } = setup(async (method) => {
      if (method !== "system.status")
        throw new BackendError("cancelled", "用户取消了管理员权限操作");
    });
    expect(await model.system("system.installCli", "安装 CLI")).toBe(false);
    expect(model.getSnapshot().notices[0]?.error).toBe(false);
    expect(model.getSnapshot().busy).toBeNull();
    model.close();
  });
});

test("端口输入拒绝小数、空值、重复和超范围，编辑原端口有效", () => {
  for (const text of ["", "0", "22.5", "-22", "65536", "2222"])
    expect(routeErrors(config.routes[0]!, text, config, 0).port).toBeDefined();
  expect(routeErrors(config.routes[0]!, "22", config, 0)).toEqual({});
});

test("搜索支持端口、名称、路径且保留原索引", () => {
  expect(filterRoutes(config, "git bash").map((r) => r.index)).toEqual([1]);
  expect(filterRoutes(config, "2222")).toHaveLength(1);
  expect(filterRoutes(config, "PWSH.EXE")).toHaveLength(1);
  expect(filterRoutes(config, "missing")).toHaveLength(0);
});
