import {
  Children,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import {
  AnimatePresence,
  Button,
  Dialog,
  DialogBackdrop,
  DialogPopup,
  DialogPortal,
  motion,
  useGpuix,
  useWindowSize,
  type KeyEvent,
  type PublicInstance,
  type StyleDesc,
} from "@gpuix/react";
import { BackendError, type Config, type Route } from "./backend";
import { AppModel, dirty, filterRoutes, routeErrors } from "./model";

const C = {
  bg: "#101722",
  panel: "#182231",
  input: "#101925",
  border: "#2B3B50",
  text: "#E8EEF6",
  muted: "#96A9BF",
  accent: "#70BAFF",
  primary: "#2568B5",
  hover: "#263A52",
  green: "#78D8AE",
  red: "#FF929B",
  amber: "#F4CA79",
};
const row: StyleDesc = {
  display: "flex",
  flexDirection: "row",
  alignItems: "center",
  gap: 10,
};
const column: StyleDesc = { display: "flex", flexDirection: "column", gap: 10 };
const panel: StyleDesc = {
  ...column,
  padding: 20,
  borderRadius: 12,
  backgroundColor: C.panel,
  borderWidth: 1,
  borderColor: C.border,
};

function Text({
  children,
  color = C.text,
  size = 14,
  style,
}: {
  children: ReactNode;
  color?: string;
  size?: number;
  style?: StyleDesc;
}) {
  const parts = Children.toArray(children);
  const content = parts.every(
    (part) => typeof part === "string" || typeof part === "number",
  )
    ? parts.join("")
    : children;
  return <text style={{ color, fontSize: size, ...style }}>{content}</text>;
}

function Action({
  children,
  onClick,
  disabled,
  primary,
  danger,
  testId,
}: {
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  primary?: boolean;
  danger?: boolean;
  testId?: string;
}) {
  return (
    <Button
      testId={testId}
      disabled={disabled}
      onClick={onClick}
      style={{
        ...row,
        justifyContent: "center",
        paddingLeft: 13,
        paddingRight: 13,
        height: 34,
        flexShrink: 0,
        borderRadius: 7,
        borderWidth: 1,
        borderColor: primary ? C.primary : C.border,
        backgroundColor: primary ? C.primary : C.panel,
        opacity: disabled ? 0.4 : 1,
        cursor: disabled ? "default" : "pointer",
        hover: { backgroundColor: primary ? "#327DCD" : C.hover },
        active: { opacity: 0.65 },
        focusVisible: {
          outlineWidth: 2,
          outlineColor: C.accent,
          outlineOffset: 2,
        },
      }}
    >
      <Text color={danger ? C.red : C.text} size={13}>
        {children}
      </Text>
    </Button>
  );
}

function Field({
  label,
  value,
  change,
  error,
  hint,
  testId,
  autoFocus,
  readOnly,
  inputRef,
  style,
}: {
  label: string;
  value: string;
  change: (value: string) => void;
  error?: string;
  hint?: string;
  testId?: string;
  autoFocus?: boolean;
  readOnly?: boolean;
  inputRef?: React.Ref<PublicInstance>;
  style?: StyleDesc;
}) {
  return (
    <div style={{ ...column, gap: 5, minWidth: 0, flexGrow: 1, ...style }}>
      <Text size={12} color={C.muted}>
        {label}
      </Text>
      <input
        ref={inputRef}
        testId={testId}
        value={value}
        autoFocus={autoFocus}
        readOnly={readOnly}
        aria-label={label}
        aria-invalid={!!error}
        onChange={(event) => change(event.value ?? "")}
        theme={{ caret: C.accent }}
        style={{
          height: 36,
          paddingLeft: 10,
          paddingRight: 10,
          backgroundColor: C.input,
          borderWidth: 1,
          borderRadius: 6,
          borderColor: error ? C.red : C.border,
          color: C.text,
          fontSize: 13,
          focusVisible: { borderColor: C.accent },
        }}
      />
      {(error || hint) && (
        <Text color={error ? C.red : C.muted} size={11}>
          {error || hint}
        </Text>
      )}
    </div>
  );
}

function Modal({
  open,
  close,
  title,
  children,
  footer,
  testId,
  busy = false,
}: {
  open: boolean;
  close: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  testId?: string;
  busy?: boolean;
}) {
  const size = useWindowSize();
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && !busy) close();
      }}
      disablePointerDismissal
    >
      <DialogPortal>
        <DialogBackdrop style={{ backgroundColor: "#000000AA" }} />
        <DialogPopup
          testId={testId}
          style={{
            width: Math.min(600, size.width - 36),
            maxHeight: Math.max(260, size.height - 42),
            ...panel,
            padding: 22,
            overflow: "hidden",
          }}
        >
          <Text size={20} style={{ flexShrink: 0 }}>
            {title}
          </Text>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            style={{
              ...column,
              position: "relative",
              gap: 16,
              minHeight: 0,
              flexShrink: 1,
              maxHeight: Math.max(140, size.height - 200),
              overflowY: "scroll",
              scrollbarColor: C.border,
            }}
          >
            {children}
          </motion.div>
          {footer && <div style={{ flexShrink: 0 }}>{footer}</div>}
        </DialogPopup>
      </DialogPortal>
    </Dialog>
  );
}

function Editor({
  model,
  config,
  index,
  close,
}: {
  model: AppModel;
  config: Config;
  index: number | null;
  close: () => void;
}) {
  const initial =
    index === null
      ? {
          port: 2224,
          name: "",
          shell: "",
          interactiveTemplate: '"{shell}" -l',
          commandTemplate: '"{shell}" "{tmpfile}"',
          tmpFileExt: ".sh",
          default: false,
        }
      : config.routes[index]!;
  const [route, setRoute] = useState<Route>({ ...initial });
  const [port, setPort] = useState(String(initial.port));
  const [submitted, setSubmitted] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [touched, setTouched] = useState<Record<string, boolean>>({});
  const state = useSyncExternalStore(model.subscribe, model.getSnapshot);
  const errors = routeErrors(route, port, config, index);
  const visibleError = (field: string) =>
    submitted || touched[field] ? errors[field] : undefined;
  const update = (field: keyof Route, value: string) => {
    setRoute((previous) => ({ ...previous, [field]: value }));
    setTouched((previous) => ({ ...previous, [field]: true }));
  };
  const save = async () => {
    setSubmitError(null);
    setSubmitted(true);
    if (Object.keys(errors).length) return;
    if (await model.acceptRoute(index, { ...route, port: Number(port) }))
      close();
    else
      setSubmitError(
        model.getSnapshot().notices.at(-1)?.message ?? "校验失败，请重试",
      );
  };
  return (
    <Modal
      open
      close={close}
      title={index === null ? "添加端口路由" : "编辑端口路由"}
      testId="route-dialog"
      busy={!!state.busy}
      footer={
        <div style={{ ...row, justifyContent: "flex-end" }}>
          <Action onClick={close} disabled={!!state.busy}>
            取消
          </Action>
          <Action
            testId="apply-route"
            primary
            onClick={() => void save()}
            disabled={!!state.busy}
          >
            {state.busy ? "正在校验…" : "应用修改"}
          </Action>
        </div>
      }
    >
      {submitError && (
        <Text color={C.red} size={12}>
          {submitError}
        </Text>
      )}
      <div style={row}>
        <Field
          label="端口"
          testId="route-port"
          style={{ flexBasis: 0 }}
          value={port}
          autoFocus
          change={(value) => {
            setPort(value);
            setTouched((p) => ({ ...p, port: true }));
          }}
          error={visibleError("port")}
        />
        <Field
          label="名称"
          testId="route-name"
          style={{ flexBasis: 0 }}
          value={route.name}
          change={(value) => update("name", value)}
          error={visibleError("name")}
        />
      </div>
      <Field
        label="Shell 路径"
        testId="route-shell"
        value={route.shell}
        change={(value) => update("shell", value)}
        error={visibleError("shell")}
      />
      <Field
        label="交互式模板"
        testId="route-interactive"
        value={route.interactiveTemplate}
        change={(value) => update("interactiveTemplate", value)}
        error={visibleError("interactiveTemplate")}
        hint="{shell} 表示 Shell 路径"
      />
      <Field
        label="命令模板"
        testId="route-command"
        value={route.commandTemplate}
        change={(value) => update("commandTemplate", value)}
        error={visibleError("commandTemplate")}
        hint="{tmpfile} 表示临时脚本路径"
      />
      <Field
        label="临时文件扩展名"
        testId="route-extension"
        value={route.tmpFileExt}
        change={(value) => update("tmpFileExt", value)}
        error={visibleError("tmpFileExt")}
      />
      <Button
        testId="route-default"
        onClick={() => setRoute((r) => ({ ...r, default: !r.default }))}
        aria-pressed={route.default}
        style={{
          ...row,
          padding: 8,
          borderRadius: 6,
          focusVisible: { outlineColor: C.accent, outlineWidth: 2 },
        }}
      >
        <Text color={route.default ? C.accent : C.muted}>
          {route.default ? "☑" : "☐"} 设为默认路由
        </Text>
      </Button>
      <Text size={12} color={C.muted}>
        应用修改后仍需保存配置。
      </Text>
    </Modal>
  );
}

type Confirmation = {
  title: string;
  message: string;
  label: string;
  danger?: boolean;
  action: () => void;
};

export function App({
  model,
  onReady,
}: {
  model: AppModel;
  onReady?: () => void;
}) {
  const state = useSyncExternalStore(model.subscribe, model.getSnapshot);
  const [query, setQuery] = useState("");
  const [editor, setEditor] = useState<{ index: number | null } | null>(null);
  const [confirm, setConfirm] = useState<Confirmation | null>(null);
  const [tab, setTab] = useState<"routes" | "system">("routes");
  const search = useRef<PublicInstance>(null);
  const content = useRef<PublicInstance>(null);
  const ready = useRef(false);
  const renderer = useGpuix();
  const size = useWindowSize();
  const compact = size.width < 760;
  const busy = !!state.busy;
  const changed = dirty(state);
  const windows = state.hello?.windows ?? false;
  const config = state.config;
  const shown = config ? filterRoutes(config, query) : [];
  useEffect(() => {
    if (content.current)
      renderer.renderer?.scrollTo?.(content.current.id, 0, 0);
  }, [tab, query, renderer.renderer]);
  const reload = () => {
    if (changed)
      setConfirm({
        title: "放弃未保存的修改？",
        message: "重新加载会用磁盘上的配置替换当前草稿。",
        label: "放弃并重新加载",
        danger: true,
        action: () => {
          void model.reload();
        },
      });
    else void model.reload();
  };
  const newRoute = () => {
    if (config && !busy && !editor && !confirm) {
      setTab("routes");
      setEditor({ index: null });
    }
  };
  const keys = (event: KeyEvent) => {
    if (event.defaultPrevented || editor || confirm || busy) return;
    const command = event.modifiers?.ctrl || event.modifiers?.cmd;
    if (!command) return;
    if (event.key === "s") {
      event.preventDefault();
      void model.save();
    }
    if (event.key === "n") {
      event.preventDefault();
      newRoute();
    }
    if (event.key === "f") {
      event.preventDefault();
      setTab("routes");
      setTimeout(() => {
        if (search.current)
          renderer.renderer?.focusElement?.(search.current.id);
      }, 0);
    }
  };
  useEffect(() => {
    void model.initialize();
    const initialCheck = setTimeout(() => {
      void model.checkUpdate(true);
    }, 30_000);
    const timer = setInterval(
      () => {
        void model.checkUpdate(true);
      },
      24 * 60 * 60 * 1000,
    );
    return () => {
      clearInterval(timer);
      clearTimeout(initialCheck);
    };
  }, [model]);
  const systemAction = (method: string, label: string, message: string) =>
    setConfirm({
      title: label,
      message,
      label: "继续",
      action: () => {
        void model.system(method, label);
      },
    });

  return (
    <div
      testId="app"
      onKeyDown={keys}
      onLayout={() => {
        if (!ready.current) {
          ready.current = true;
          onReady?.();
        }
      }}
      style={{
        ...column,
        gap: 0,
        height: "100%",
        width: "100%",
        backgroundColor: C.bg,
        fontFamily:
          process.platform === "win32" ? "Microsoft YaHei UI" : "PingFang SC",
      }}
    >
      <div
        style={{
          ...row,
          padding: compact ? 18 : 24,
          justifyContent: "space-between",
          borderBottomWidth: 1,
          borderColor: C.border,
        }}
      >
        <div style={{ ...column, gap: 5 }}>
          <Text size={24} style={{ fontWeight: 700 }}>
            SSH Router
          </Text>
          <Text size={12} color={C.muted}>
            Windows OpenSSH 端口路由管理
          </Text>
        </div>
        <div style={{ ...row, flexWrap: "wrap", justifyContent: "flex-end" }}>
          <Text size={12} color={changed ? C.amber : C.green}>
            {changed ? "● 未保存" : config ? "✓ 已保存" : "配置未加载"}
          </Text>
          <Action testId="reload" onClick={reload} disabled={busy}>
            重新加载
          </Action>
          <Action
            testId="save"
            primary
            onClick={() => void model.save()}
            disabled={!changed || !config || busy}
          >
            {state.busy === "正在保存" ? "保存中…" : "保存配置"}
          </Action>
        </div>
      </div>
      <div
        style={{
          ...row,
          paddingLeft: compact ? 18 : 24,
          paddingTop: 12,
          paddingBottom: 12,
          borderBottomWidth: 1,
          borderColor: C.border,
        }}
      >
        <Action
          testId="tab-routes"
          primary={tab === "routes"}
          onClick={() => setTab("routes")}
        >
          端口路由
        </Action>
        <Action
          testId="tab-system"
          primary={tab === "system"}
          onClick={() => setTab("system")}
        >
          系统与更新
        </Action>
        <div style={{ flexGrow: 1 }} />
        {!compact && (
          <Text color={C.muted} size={11}>
            Ctrl+S 保存 · Ctrl+N 新增 · Ctrl+F 搜索
          </Text>
        )}
      </div>
      <div
        ref={content}
        testId="content"
        style={{
          ...column,
          gap: 18,
          padding: compact ? 18 : 24,
          minHeight: 0,
          flexGrow: 1,
          overflowY: "scroll",
          scrollbarColor: C.border,
        }}
      >
        {state.loadError && (
          <div testId="config-error" style={panel}>
            <Text color={C.amber}>
              {state.loadError instanceof BackendError &&
              state.loadError.code === "not_found"
                ? "尚未创建配置"
                : "配置加载失败"}
            </Text>
            <Text size={12} color={C.muted}>
              {state.loadError.message}
            </Text>
            {state.loadError instanceof BackendError &&
              state.loadError.code === "not_found" && (
                <Action
                  testId="create-config"
                  onClick={() => void model.create()}
                  disabled={busy}
                  primary
                >
                  创建默认配置
                </Action>
              )}
          </div>
        )}
        {tab === "routes" ? (
          <>
            <div style={{ ...row, alignItems: "flex-end", flexWrap: "wrap" }}>
              <div style={{ ...column, gap: 6, flexGrow: 1 }}>
                <Text size={19}>端口路由</Text>
                <Text size={12} color={C.muted}>
                  管理 SSH 端口对应的 Shell 环境
                </Text>
              </div>
              <div style={{ width: compact ? 180 : 240 }}>
                <Field
                  label="搜索端口、名称或路径"
                  testId="search"
                  inputRef={search}
                  value={query}
                  change={setQuery}
                />
              </div>
              <Action
                testId="add-route"
                primary
                onClick={newRoute}
                disabled={!config || busy}
              >
                ＋ 添加路由
              </Action>
            </div>
            {config &&
              (shown.length ? (
                shown.map(({ route, index }) => (
                  <motion.div
                    key={`${index}-${route.port}`}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ duration: 0.15 }}
                    style={{
                      ...panel,
                      gap: 12,
                      hover: {
                        borderColor: "#49749A",
                        backgroundColor: "#1C2B3D",
                      },
                    }}
                  >
                    <div
                      style={{
                        ...row,
                        justifyContent: "space-between",
                        flexWrap: "wrap",
                      }}
                    >
                      <div style={{ ...row, gap: 12 }}>
                        <div
                          style={{
                            padding: 9,
                            borderRadius: 8,
                            backgroundColor: "#213E5C",
                          }}
                        >
                          <Text
                            color={C.accent}
                            size={19}
                            style={{ fontWeight: 700 }}
                          >
                            {route.port}
                          </Text>
                        </div>
                        <Text size={16}>{route.name}</Text>
                        {route.default && (
                          <Text color={C.green} size={11}>
                            默认路由
                          </Text>
                        )}
                      </div>
                      <div style={row}>
                        {!route.default && (
                          <Action
                            testId={`default-${route.port}`}
                            onClick={() => model.setDefault(index)}
                            disabled={busy}
                          >
                            设为默认
                          </Action>
                        )}
                        <Action
                          testId={`edit-${route.port}`}
                          onClick={() => setEditor({ index })}
                          disabled={busy}
                        >
                          编辑
                        </Action>
                        <Action
                          testId={`delete-${route.port}`}
                          danger
                          disabled={busy || config.routes.length <= 1}
                          onClick={() =>
                            setConfirm({
                              title: `删除路由 ${route.port}？`,
                              message: `将从草稿中移除“${route.name}”。${route.default ? "默认路由将切换为其余第一条路由。" : ""}保存配置后生效。`,
                              label: "删除路由",
                              danger: true,
                              action: () => model.removeRoute(index),
                            })
                          }
                        >
                          删除
                        </Action>
                      </div>
                    </div>
                    <Text
                      size={12}
                      color={C.muted}
                      style={{ whiteSpace: "normal" }}
                    >
                      {route.shell}
                    </Text>
                  </motion.div>
                ))
              ) : (
                <div style={panel}>
                  <Text>未找到匹配路由</Text>
                  <Action onClick={() => setQuery("")}>清空搜索</Action>
                </div>
              ))}
            {config && (
              <div style={panel}>
                <Text size={17}>SFTP 命令</Text>
                <Field
                  label="SFTP 子系统启动命令"
                  testId="sftp"
                  readOnly={busy}
                  value={config.sftpCommand}
                  change={(sftpCommand) =>
                    model.edit({ ...config, sftpCommand })
                  }
                  error={
                    !config.sftpCommand.trim() ? "SFTP 命令不能为空" : undefined
                  }
                />
                <Text color={C.muted} size={11}>
                  保存配置后，下一次 SSH 连接立即生效。
                </Text>
              </div>
            )}
          </>
        ) : (
          <>
            <div style={panel}>
              <div style={{ ...row, justifyContent: "space-between" }}>
                <Text size={19}>安装状态</Text>
                <Action
                  testId="refresh-status"
                  onClick={() => void model.refreshStatus()}
                  disabled={busy || !windows}
                >
                  刷新状态
                </Action>
              </div>
              {!windows && (
                <Text color={C.amber} size={12}>
                  当前平台可编辑本地配置；系统管理与更新需要 Windows。
                </Text>
              )}
              {state.statusError && (
                <Text color={C.red} size={12}>
                  {state.statusError}
                </Text>
              )}
              {state.status &&
                [
                  {
                    name: "CLI 部署",
                    ok: state.status.cliDeployed && state.status.cliUpToDate,
                    detail: state.status.cliDeployed
                      ? state.status.cliUpToDate
                        ? "已是最新"
                        : "需要更新"
                      : "未安装",
                  },
                  {
                    name: "DefaultShell",
                    ok: state.status.defaultShellSet,
                    detail: state.status.defaultShell || "尚未设置",
                  },
                  {
                    name: "配置文件",
                    ok: state.status.configExists,
                    detail: state.status.configExists ? "已创建" : "尚未创建",
                  },
                  {
                    name: "sshd 服务",
                    ok: state.status.sshdRunning,
                    detail: state.status.sshdDetail,
                  },
                ].map((item) => (
                  <div
                    key={item.name}
                    style={{
                      ...row,
                      justifyContent: "space-between",
                      paddingTop: 7,
                      paddingBottom: 7,
                    }}
                  >
                    <Text size={13}>{item.name}</Text>
                    <Text size={12} color={item.ok ? C.green : C.amber}>
                      {item.detail}
                    </Text>
                  </div>
                ))}
              {state.status?.cliDeployed && !state.status.cliUpToDate && (
                <Text size={12} color={C.amber}>
                  已部署 CLI 与安装包不一致，可点击下方按钮更新。
                </Text>
              )}
              <div style={{ ...row, flexWrap: "wrap" }}>
                <Action
                  testId="install-cli"
                  disabled={busy || !windows}
                  onClick={() =>
                    systemAction(
                      "system.installCli",
                      "安装 / 更新 CLI",
                      "将更新 C:\\ProgramData\\ssh 中的 CLI，Windows 会请求管理员权限。",
                    )
                  }
                >
                  安装 / 更新 CLI
                </Action>
                <Action
                  disabled={busy || !windows}
                  onClick={() =>
                    systemAction(
                      "system.setDefaultShell",
                      "设置 DefaultShell",
                      "将修改 OpenSSH 的 DefaultShell 注册表项，Windows 会请求管理员权限。",
                    )
                  }
                >
                  设置 DefaultShell
                </Action>
                <Action
                  disabled={busy || !windows}
                  onClick={() =>
                    systemAction(
                      "system.restartSshd",
                      "重启 sshd",
                      "将重启 SSH 服务，可能中断现有连接。",
                    )
                  }
                >
                  重启 sshd
                </Action>
              </div>
            </div>
            <div style={panel}>
              <Text size={19}>应用更新</Text>
              <Text color={C.muted} size={12}>
                当前版本 {state.hello?.version ?? "…"}
              </Text>
              {state.candidate && (
                <>
                  <Text color={C.green}>
                    可更新至 {state.candidate.version}
                  </Text>
                  {state.candidate.notes && (
                    <Text color={C.muted} size={12}>
                      {state.candidate.notes}
                    </Text>
                  )}
                </>
              )}
              {state.progress && (
                <Text color={C.accent} size={12}>
                  {state.progress.totalBytes
                    ? `已下载 ${Math.min(100, Math.round((state.progress.downloadedBytes / state.progress.totalBytes) * 100))}%`
                    : `已下载 ${(state.progress.downloadedBytes / 1048576).toFixed(1)} MiB`}
                </Text>
              )}
              <div style={row}>
                <Action
                  testId="check-update"
                  onClick={() => void model.checkUpdate()}
                  disabled={busy || !windows}
                >
                  检查更新
                </Action>
                {state.candidate && (
                  <Action
                    primary
                    disabled={busy}
                    onClick={() =>
                      setConfirm({
                        title: "安装更新？",
                        message: changed
                          ? "当前有未保存修改。安装更新将退出应用，请先保存，或确认放弃修改后继续。"
                          : "下载并验证签名后，应用会退出并安装更新。",
                        label: changed ? "放弃修改并更新" : "下载并安装",
                        danger: changed,
                        action: () => {
                          void model.installUpdate();
                        },
                      })
                    }
                  >
                    下载并安装
                  </Action>
                )}
              </div>
            </div>
          </>
        )}
      </div>
      <div
        style={{
          ...row,
          paddingLeft: 20,
          paddingRight: 20,
          height: 32,
          flexShrink: 0,
          borderTopWidth: 1,
          borderColor: C.border,
          justifyContent: "space-between",
        }}
      >
        <Text size={11} color={state.busy ? C.accent : C.muted}>
          {state.busy
            ? `${state.busy}…`
            : `${config?.routes.length ?? 0} 条路由${windows ? " · 关闭窗口后保留在托盘" : ""}`}
        </Text>
        {!compact && (
          <Text size={10} color={C.muted}>
            {state.hello?.configPath ?? ""}
          </Text>
        )}
      </div>
      {editor && config && (
        <Editor
          model={model}
          config={config}
          index={editor.index}
          close={() => setEditor(null)}
        />
      )}
      <Modal
        open={!!confirm}
        close={() => setConfirm(null)}
        title={confirm?.title ?? ""}
        testId="confirm-dialog"
      >
        <Text size={14} color={C.muted}>
          {confirm?.message}
        </Text>
        <div style={{ ...row, justifyContent: "flex-end" }}>
          <Action testId="confirm-cancel" onClick={() => setConfirm(null)}>
            取消
          </Action>
          <Action
            testId="confirm-accept"
            primary
            danger={confirm?.danger}
            onClick={() => {
              const action = confirm?.action;
              setConfirm(null);
              action?.();
            }}
          >
            {confirm?.label ?? "确定"}
          </Action>
        </div>
      </Modal>
      <div
        style={{
          ...column,
          position: "absolute",
          bottom: 48,
          right: 16,
          width: Math.min(370, size.width - 32),
          gap: 8,
          pointerEvents: "none",
        }}
      >
        <AnimatePresence>
          {state.notices.map((notice) => (
            <motion.div
              key={notice.id}
              initial={{ opacity: 0, right: -20 }}
              animate={{ opacity: 1, right: 0 }}
              exit={{ opacity: 0, right: -20 }}
              transition={{ duration: 0.18 }}
              style={{
                ...panel,
                position: "relative",
                padding: 12,
                borderColor: notice.error ? C.red : C.green,
                pointerEvents: "none",
              }}
            >
              <div style={row}>
                <div style={{ flexGrow: 1, minWidth: 0 }}>
                  <Text size={13} color={notice.error ? C.red : C.text}>
                    {notice.message}
                  </Text>
                </div>
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
}
