import {
  BackendError,
  type Backend,
  type Candidate,
  type Config,
  type Hello,
  type Progress,
  type Route,
  type Status,
} from "./backend";

export interface State {
  hello: Hello | null;
  config: Config | null;
  saved: Config | null;
  busy: string | null;
  loadError: Error | null;
  status: Status | null;
  statusError: string | null;
  candidate: Candidate | null;
  progress: Progress | null;
  notices: { id: number; message: string; error: boolean }[];
}

export function dirty(state: Pick<State, "config" | "saved">): boolean {
  return JSON.stringify(state.config) !== JSON.stringify(state.saved);
}

export function routeErrors(
  route: Route,
  portText: string,
  config: Config,
  index: number | null,
): Record<string, string> {
  const errors: Record<string, string> = {};
  if (
    !/^\d+$/.test(portText) ||
    Number(portText) < 1 ||
    Number(portText) > 65535
  )
    errors.port = "端口须为 1–65535 的整数";
  else if (
    config.routes.some((r, i) => i !== index && r.port === Number(portText))
  )
    errors.port = "此端口已有路由";
  for (const [field, label] of [
    ["name", "名称"],
    ["shell", "Shell 路径"],
    ["interactiveTemplate", "交互式模板"],
    ["commandTemplate", "命令模板"],
  ] as const) {
    if (!route[field].trim()) errors[field] = `${label}不能为空`;
  }
  if (!/^\.[^/\\]+$/.test(route.tmpFileExt.trim()))
    errors.tmpFileExt = "请输入 .sh 或 .ps1 等扩展名";
  return errors;
}

export function replaceRoute(
  config: Config,
  index: number | null,
  route: Route,
): Config {
  const routes = config.routes.map((r) => ({ ...r }));
  if (index === null) routes.push(route);
  else routes[index] = route;
  const target = index ?? routes.length - 1;
  if (route.default || routes.length === 1)
    routes.forEach((r, i) => {
      r.default = i === target;
    });
  return { ...config, routes };
}

export function filterRoutes(config: Config, query: string) {
  const text = query.trim().toLocaleLowerCase();
  return config.routes
    .map((route, index) => ({ route, index }))
    .filter(({ route }) =>
      `${route.port} ${route.name} ${route.shell}`
        .toLocaleLowerCase()
        .includes(text),
    );
}

export class AppModel {
  private state: State = {
    hello: null,
    config: null,
    saved: null,
    busy: null,
    loadError: null,
    status: null,
    statusError: null,
    candidate: null,
    progress: null,
    notices: [],
  };
  private listeners = new Set<() => void>();
  private noticeId = 0;
  private timers = new Set<ReturnType<typeof setTimeout>>();
  constructor(readonly backend: Backend) {
    backend.onProgress = (progress) => this.patch({ progress });
  }
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  patch(patch: Partial<State>) {
    this.state = { ...this.state, ...patch };
    this.listeners.forEach((listener) => listener());
  }
  edit(config: Config) {
    this.patch({ config });
  }
  notice(message: string, error = false) {
    const id = ++this.noticeId;
    this.patch({
      notices: [...this.state.notices.slice(-3), { id, message, error }],
    });
    const timer = setTimeout(
      () => {
        this.dismiss(id);
        this.timers.delete(timer);
      },
      error ? 9000 : 4500,
    );
    timer.unref?.();
    this.timers.add(timer);
  }
  dismiss(id: number) {
    this.patch({
      notices: this.state.notices.filter((notice) => notice.id !== id),
    });
  }

  private async run(
    label: string,
    task: () => Promise<void>,
  ): Promise<boolean> {
    if (this.state.busy) return false;
    this.patch({ busy: label });
    try {
      await task();
      return true;
    } catch (error) {
      this.notice(
        error instanceof Error ? error.message : String(error),
        !(error instanceof BackendError && error.code === "cancelled"),
      );
      return false;
    } finally {
      this.patch({ busy: null });
    }
  }

  async initialize() {
    await this.run("正在加载", async () => {
      try {
        const hello = await this.backend.request<Hello>("hello");
        if (hello.protocol !== 1) throw new Error("界面与业务进程版本不兼容");
        this.patch({ hello });
        await this.loadConfig();
        if (hello.windows) await this.readStatus();
        if (this.state.status?.cliDeployed && !this.state.status.cliUpToDate) {
          this.notice("已部署 CLI 需要更新，请在“系统与更新”页同步");
        }
      } catch (error) {
        this.patch({
          loadError: error instanceof Error ? error : new Error(String(error)),
        });
        throw error;
      }
    });
  }

  private async loadConfig() {
    try {
      const config = await this.backend.request<Config>("config.load");
      this.patch({ config, saved: structuredClone(config), loadError: null });
    } catch (error) {
      this.patch({
        loadError: error instanceof Error ? error : new Error(String(error)),
      });
    }
  }

  async reload() {
    return this.run("正在重新加载", async () => {
      await this.loadConfig();
    });
  }
  async create() {
    return this.run("正在创建配置", async () => {
      const config = await this.backend.request<Config>("config.create");
      this.patch({ config, saved: structuredClone(config), loadError: null });
      this.notice("默认配置已创建");
      if (this.state.hello?.windows) await this.readStatus();
    });
  }
  async save() {
    if (!this.state.config || !dirty(this.state)) return false;
    const snapshot = structuredClone(this.state.config);
    return this.run("正在保存", async () => {
      await this.backend.request("config.save", snapshot);
      this.patch({ saved: snapshot });
      this.notice("配置已保存，下次 SSH 连接生效");
    });
  }

  async acceptRoute(index: number | null, route: Route) {
    if (!this.state.config) return false;
    const next = replaceRoute(this.state.config, index, route);
    return this.run("正在校验", async () => {
      await this.backend.request("config.validate", next);
      this.edit(next);
    });
  }
  removeRoute(index: number) {
    if (
      !this.state.config ||
      this.state.busy ||
      this.state.config.routes.length <= 1
    )
      return;
    const routes = this.state.config.routes
      .filter((_, i) => i !== index)
      .map((route) => ({ ...route }));
    if (!routes.some((route) => route.default)) routes[0]!.default = true;
    this.edit({ ...this.state.config, routes });
    this.notice("路由已移除，保存配置后生效");
  }
  setDefault(index: number) {
    if (!this.state.config || this.state.busy) return;
    this.edit({
      ...this.state.config,
      routes: this.state.config.routes.map((route, i) => ({
        ...route,
        default: index === i,
      })),
    });
  }

  private async readStatus() {
    try {
      this.patch({
        status: await this.backend.request<Status>("system.status"),
        statusError: null,
      });
    } catch (error) {
      this.patch({
        statusError: error instanceof Error ? error.message : String(error),
      });
    }
  }
  async refreshStatus() {
    return this.run("正在刷新状态", () => this.readStatus());
  }
  async system(method: string, label: string) {
    return this.run(label, async () => {
      try {
        const result = await this.backend.request<{ message: string }>(method);
        this.notice(result.message);
      } finally {
        await this.readStatus();
      }
    });
  }
  async checkUpdate(silent = false) {
    if (!this.state.hello?.windows || this.state.busy) return false;
    if (silent) {
      this.patch({ busy: "正在检查更新" });
      try {
        this.patch({
          candidate: await this.backend.request<Candidate | null>(
            "update.check",
          ),
        });
      } catch {
        /* 自动检查失败保持静默。 */
      } finally {
        this.patch({ busy: null });
      }
      return true;
    }
    return this.run("正在检查更新", async () => {
      const candidate = await this.backend.request<Candidate | null>(
        "update.check",
      );
      this.patch({ candidate });
      this.notice(
        candidate ? `发现新版本 ${candidate.version}` : "当前已是最新版本",
      );
    });
  }
  async installUpdate() {
    return this.run("正在下载并验证更新", async () => {
      this.patch({ progress: null });
      await this.backend.request("update.install");
      this.close();
      process.exit(0);
    });
  }
  close() {
    this.timers.forEach(clearTimeout);
    this.backend.close();
  }
}
