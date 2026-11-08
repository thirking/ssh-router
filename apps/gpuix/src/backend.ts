import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { createInterface } from "node:readline";

export interface Route {
  port: number;
  name: string;
  shell: string;
  interactiveTemplate: string;
  commandTemplate: string;
  tmpFileExt: string;
  default: boolean;
}

export interface Config {
  routes: Route[];
  sftpCommand: string;
}
export interface Hello {
  protocol: number;
  version: string;
  windows: boolean;
  configPath: string;
}
export interface Status {
  cliDeployed: boolean;
  cliUpToDate: boolean;
  cliPath: string;
  defaultShellSet: boolean;
  defaultShell: string;
  configExists: boolean;
  sshdRunning: boolean;
  sshdDetail: string;
}
export interface Candidate {
  version: string;
  notes: string | null;
}
export interface Progress {
  event: "update.progress";
  downloadedBytes: number;
  totalBytes: number | null;
}
export interface Backend {
  request<T>(method: string, params?: unknown): Promise<T>;
  onProgress?: (progress: Progress) => void;
  close(): void;
}

export class BackendError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

export function resourcePath(name: string): string {
  return join(dirname(process.execPath), "resources", name);
}

export function backendPath(): string {
  if (process.env.SSH_ROUTER_BACKEND)
    return resolve(process.env.SSH_ROUTER_BACKEND);
  const name = `ssh-router-backend${process.platform === "win32" ? ".exe" : ""}`;
  const installed = join(dirname(process.execPath), name);
  return existsSync(installed)
    ? installed
    : resolve(import.meta.dir, "../../../target/debug", name);
}

export class ProcessBackend implements Backend {
  private child: ChildProcessWithoutNullStreams;
  private sequence = 0;
  private failure: Error | null = null;
  private pending = new Map<
    number,
    {
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
      timer: ReturnType<typeof setTimeout>;
    }
  >();
  onProgress?: (progress: Progress) => void;

  constructor(path = backendPath()) {
    const args = process.env.SSH_ROUTER_CONFIG
      ? ["--config", resolve(process.env.SSH_ROUTER_CONFIG)]
      : [];
    this.child = spawn(path, args, {
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    this.child.stderr.on("data", (data) => process.stderr.write(data));
    const lines = createInterface({ input: this.child.stdout });
    lines.on("line", (line) => {
      try {
        const message = JSON.parse(line);
        if (message.event === "update.progress") {
          this.onProgress?.(message);
          return;
        }
        if (!Number.isSafeInteger(message.id)) throw new Error("后端协议错误");
        const pending = this.pending.get(message.id);
        if (!pending) return;
        clearTimeout(pending.timer);
        this.pending.delete(message.id);
        if (message.error)
          pending.reject(
            new BackendError(message.error.code, message.error.message),
          );
        else pending.resolve(message.result);
      } catch (error) {
        this.fail(error instanceof Error ? error : new Error(String(error)));
      }
    });
    this.child.on("error", (error) => this.fail(error));
    this.child.stdin.on("error", (error) => this.fail(error));
    this.child.on("exit", () =>
      this.fail(new Error("业务进程已退出，请重新启动应用")),
    );
  }

  private fail(error: Error) {
    this.failure = error;
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }

  request<T>(method: string, params?: unknown): Promise<T> {
    if (this.failure) return Promise.reject(this.failure);
    return new Promise((resolve, reject) => {
      const id = ++this.sequence;
      const timer = setTimeout(
        () => {
          this.fail(new Error("操作超时，请重新启动应用后核对系统状态"));
          this.child.kill();
        },
        method === "update.install" ? 960_000 : 65_000,
      );
      this.pending.set(id, {
        resolve: (value) => resolve(value as T),
        reject,
        timer,
      });
      this.child.stdin.write(
        JSON.stringify({ id, method, params: params ?? null }) + "\n",
        (error) => {
          if (error) this.fail(error);
        },
      );
    });
  }

  close() {
    this.fail(new Error("应用已关闭"));
    this.child.stdin.end();
    this.child.kill();
  }
}
