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
  type EventPayload,
  type KeyEvent,
  type PublicInstance,
  type StyleDesc,
} from "@gpuix/react";
import { BackendError, type Config, type Route } from "./backend";
import { AppModel, dirty, filterRoutes, routeErrors } from "./model";
import type { Desktop, TitlebarRegion } from "./desktop";

const C = {
  bg: "#282C33",
  panel: "#2F343E",
  title: "#3B414D",
  input: "#282C33",
  border: "#464B57",
  divider: "#363C46",
  text: "#DCE0E5",
  muted: "#A9AFBC",
  accent: "#74ADE8",
  primary: "#293B5B",
  hover: "#363C46",
  green: "#A1C181",
  red: "#D07277",
  amber: "#DEC184",
};
const row: StyleDesc = {
  display: "flex",
  flexDirection: "row",
  alignItems: "center",
  gap: 6,
};
const column: StyleDesc = { display: "flex", flexDirection: "column", gap: 8 };
const panel: StyleDesc = {
  ...column,
  padding: 12,
  borderRadius: 3,
  backgroundColor: C.panel,
  borderWidth: 1,
  borderColor: C.border,
};

function Text({
  children,
  color = C.text,
  size = 12,
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
  return (
    <text style={{ color, fontSize: size, fontWeight: 400, ...style }}>
      {content}
    </text>
  );
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
        paddingLeft: 8,
        paddingRight: 8,
        height: 26,
        flexShrink: 0,
        borderRadius: 3,
        borderWidth: 1,
        borderColor: primary ? "#47679E" : C.border,
        backgroundColor: primary ? C.primary : C.panel,
        opacity: disabled ? 0.4 : 1,
        cursor: disabled ? "default" : "pointer",
        hover: { backgroundColor: primary ? "#354C70" : C.hover },
        active: { opacity: 0.65 },
        focusVisible: {
          outlineWidth: 2,
          outlineColor: C.accent,
          outlineOffset: 2,
        },
      }}
    >
      <Text color={danger ? C.red : primary ? C.accent : C.text} size={11}>
        {children}
      </Text>
    </Button>
  );
}

function WindowTitlebar({
  desktop,
  blocked,
}: {
  desktop: Desktop;
  blocked: boolean;
}) {
  const { renderer } = useGpuix();
  const dragRegion = useRef<TitlebarRegion | null>(null);
  const maximized = desktop.isWindowMaximized();
  const updateDragRegion = ({ x, y, width, height }: EventPayload) => {
    if (
      x === undefined ||
      y === undefined ||
      width === undefined ||
      height === undefined
    )
      return;
    dragRegion.current = { x, y, width, height };
    desktop.setTitlebarRegion(blocked ? null : dragRegion.current);
  };
  useEffect(() => {
    desktop.setTitlebarRegion(blocked ? null : dragRegion.current);
    return () => desktop.setTitlebarRegion(null);
  }, [desktop, blocked]);
  const controls = [
    {
      id: "minimize",
      label: "最小化",
      glyph: "\uE921",
      action: () => renderer?.minimizeWindow?.(),
    },
    {
      id: "maximize",
      label: maximized ? "还原" : "最大化",
      glyph: maximized ? "\uE923" : "\uE922",
      action: () => desktop.toggleMaximizeWindow(),
    },
    {
      id: "close",
      label: "关闭到托盘",
      glyph: "\uE8BB",
      action: () => desktop.closeWindow(),
    },
  ];
  return (
    <div
      testId="window-titlebar"
      style={{
        ...row,
        gap: 0,
        height: 32,
        flexShrink: 0,
        backgroundColor: C.title,
        borderBottomWidth: 1,
        borderColor: C.divider,
      }}
    >
      <div
        testId="window-drag-region"
        onLayout={updateDragRegion}
        style={{ ...row, height: "100%", flexGrow: 1, paddingLeft: 12 }}
      >
        <Text size={11} color={C.muted}>
          SSH Router
        </Text>
      </div>
      {controls.map((control) => (
        <Button
          key={control.id}
          testId={`window-${control.id}`}
          aria-label={control.label}
          onClick={control.action}
          style={{
            ...row,
            justifyContent: "center",
            width: 40,
            height: "100%",
            flexShrink: 0,
            hover: {
              backgroundColor: control.id === "close" ? "#C42B3B" : C.border,
            },
            active: { opacity: 0.65 },
            focusVisible: {
              outlineWidth: 1,
              outlineColor: C.accent,
              outlineOffset: -2,
            },
          }}
        >
          <Text size={10} style={{ fontFamily: "Segoe MDL2 Assets" }}>
            {control.glyph}
          </Text>
        </Button>
      ))}
    </div>
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
  hideLabel = false,
  placeholder,
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
  hideLabel?: boolean;
  placeholder?: string;
}) {
  return (
    <div style={{ ...column, gap: 4, minWidth: 0, flexGrow: 1, ...style }}>
      {!hideLabel && (
        <Text size={11} color={C.muted}>
          {label}
        </Text>
      )}
      <input
        ref={inputRef}
        testId={testId}
        value={value}
        placeholder={placeholder}
        autoFocus={autoFocus}
        readOnly={readOnly}
        aria-label={label}
        aria-invalid={!!error}
        onChange={(event) => change(event.value ?? "")}
        theme={{ caret: C.accent }}
        style={{
          height: 28,
          paddingLeft: 7,
          paddingRight: 7,
          backgroundColor: C.input,
          borderWidth: 1,
          borderRadius: 3,
          borderColor: error ? C.red : C.border,
          color: C.text,
          fontSize: 12,
          fontWeight: 400,
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
            width: Math.min(520, size.width - 24),
            maxHeight: Math.max(240, size.height - 32),
            ...panel,
            padding: 14,
            overflow: "hidden",
          }}
        >
          <Text size={14} style={{ flexShrink: 0, fontWeight: 600 }}>
            {title}
          </Text>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.16, ease: "easeOut" }}
            style={{
              ...column,
              position: "relative",
              gap: 10,
              minHeight: 0,
              flexShrink: 1,
              maxHeight: Math.max(120, size.height - 150),
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
          padding: 4,
          borderRadius: 3,
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
  desktop,
}: {
  model: AppModel;
  onReady?: () => void;
  desktop?: Desktop | null;
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
  const compact = size.width < 680;
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
          process.platform === "win32" ? "Segoe UI" : "Helvetica Neue",
      }}
    >
      {desktop && (
        <WindowTitlebar desktop={desktop} blocked={!!editor || !!confirm} />
      )}
      <div
        style={{
          ...row,
          height: 40,
          paddingLeft: 12,
          paddingRight: 12,
          flexShrink: 0,
          backgroundColor: C.title,
          borderBottomWidth: 1,
          borderColor: C.divider,
        }}
      >
        {!desktop && (
          <Text size={13} style={{ fontWeight: 600 }}>
            SSH Router
          </Text>
        )}
        <div style={{ flexGrow: 1 }} />
        <Text size={10} color={changed ? C.amber : C.muted}>
          {changed ? "● 未保存" : config ? "已保存" : "加载中"}
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
      <div
        style={{
          ...row,
          gap: 0,
          height: 32,
          flexShrink: 0,
          backgroundColor: C.panel,
          borderBottomWidth: 1,
          borderColor: C.divider,
        }}
      >
        {(
          [
            { id: "routes", label: "端口路由" },
            { id: "system", label: "系统与更新" },
          ] as const
        ).map((item) => (
          <Button
            key={item.id}
            testId={`tab-${item.id}`}
            onClick={() => setTab(item.id)}
            aria-pressed={tab === item.id}
            style={{
              ...row,
              justifyContent: "center",
              height: 32,
              width: 108,
              cursor: "pointer",
              borderRightWidth: 1,
              borderBottomWidth: 2,
              borderColor: tab === item.id ? C.accent : C.divider,
              backgroundColor: tab === item.id ? C.bg : C.panel,
              hover: { backgroundColor: C.hover },
              focusVisible: {
                outlineWidth: 1,
                outlineColor: C.accent,
                outlineOffset: -2,
              },
            }}
          >
            <Text size={11} color={tab === item.id ? C.text : C.muted}>
              {item.label}
            </Text>
          </Button>
        ))}
        <div style={{ flexGrow: 1 }} />
        {!compact && (
          <Text color={C.muted} size={10} style={{ marginRight: 12 }}>
            Ctrl+S 保存 · Ctrl+N 新增
          </Text>
        )}
      </div>
      <div
        ref={content}
        testId="content"
        style={{
          ...column,
          gap: 12,
          padding: 12,
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
            <div style={{ ...row, justifyContent: "space-between" }}>
              <Text size={12} style={{ fontWeight: 600 }}>
                端口路由 · {config?.routes.length ?? 0}
              </Text>
              <Text size={10} color={C.muted}>
                端口 → Shell 环境
              </Text>
            </div>
            <div style={{ ...row, gap: 8 }}>
              <Field
                label="搜索端口、名称或路径"
                hideLabel
                placeholder="搜索端口、名称或路径…"
                testId="search"
                inputRef={search}
                value={query}
                change={setQuery}
                style={{ flexBasis: 0 }}
              />
              <Action
                testId="add-route"
                primary
                onClick={newRoute}
                disabled={!config || busy}
              >
                ＋ 添加路由
              </Action>
            </div>
            {config && (
              <div
                style={{
                  ...column,
                  gap: 0,
                  flexShrink: 0,
                  borderWidth: 1,
                  borderColor: C.divider,
                  borderRadius: 3,
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    ...row,
                    height: 26,
                    paddingLeft: 10,
                    paddingRight: 10,
                    backgroundColor: C.panel,
                    borderBottomWidth: 1,
                    borderColor: C.divider,
                  }}
                >
                  <Text size={10} color={C.muted} style={{ width: 42 }}>
                    端口
                  </Text>
                  <Text size={10} color={C.muted} style={{ flexGrow: 1 }}>
                    目标环境
                  </Text>
                  <Text size={10} color={C.muted}>
                    操作
                  </Text>
                </div>
                {shown.length ? (
                  shown.map(({ route, index }) => (
                    <motion.div
                      key={`${index}-${route.port}`}
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      transition={{ duration: 0.12 }}
                      style={{ hover: { backgroundColor: C.hover } }}
                    >
                      <div
                        testId={`route-row-${route.port}`}
                        style={{
                          ...row,
                          gap: 8,
                          minHeight: 56,
                          paddingLeft: 10,
                          paddingRight: 10,
                          paddingTop: 7,
                          paddingBottom: 7,
                          borderBottomWidth: 1,
                          borderColor: C.divider,
                        }}
                      >
                        <Text
                          color={C.accent}
                          size={12}
                          style={{
                            width: 40,
                            flexShrink: 0,
                            fontFamily:
                              process.platform === "win32"
                                ? "Consolas"
                                : "Menlo",
                          }}
                        >
                          {route.port}
                        </Text>
                        <div
                          style={{
                            ...column,
                            gap: 2,
                            flexGrow: 1,
                            flexBasis: 0,
                            minWidth: 0,
                          }}
                        >
                          <div style={{ ...row, gap: 6 }}>
                            <Text
                              size={12}
                              style={{
                                minWidth: 0,
                                flexShrink: 1,
                                whiteSpace: "nowrap",
                                textOverflow: "ellipsis",
                              }}
                            >
                              {route.name}
                            </Text>
                            {route.default && (
                              <Text
                                color={C.green}
                                size={10}
                                style={{ flexShrink: 0 }}
                              >
                                默认
                              </Text>
                            )}
                          </div>
                          <Text
                            size={10}
                            color={C.muted}
                            style={{
                              whiteSpace: "nowrap",
                              textOverflow: "ellipsis",
                              fontFamily:
                                process.platform === "win32"
                                  ? "Consolas"
                                  : "Menlo",
                            }}
                          >
                            {route.shell}
                          </Text>
                        </div>
                        <div style={{ ...row, gap: 4, flexShrink: 0 }}>
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
                    </motion.div>
                  ))
                ) : (
                  <div style={{ ...column, padding: 16 }}>
                    <Text color={C.muted}>未找到匹配路由</Text>
                    <Action onClick={() => setQuery("")}>清空搜索</Action>
                  </div>
                )}
              </div>
            )}
            {config && (
              <div style={panel}>
                <Text size={12}>SFTP 命令</Text>
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
                <Text size={12}>安装状态</Text>
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
                      paddingTop: 4,
                      paddingBottom: 4,
                    }}
                  >
                    <Text size={11} style={{ flexShrink: 0 }}>
                      {item.name}
                    </Text>
                    <Text
                      size={11}
                      color={item.ok ? C.green : C.amber}
                      style={{
                        flexGrow: 1,
                        flexBasis: 0,
                        minWidth: 0,
                        whiteSpace: "nowrap",
                        textOverflow: "ellipsis",
                        textAlign: "right",
                      }}
                    >
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
              <Text size={12}>应用更新</Text>
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
          paddingLeft: 12,
          paddingRight: 12,
          height: 24,
          backgroundColor: C.panel,
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
        <Text size={12} color={C.muted}>
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
          bottom: 34,
          right: 10,
          width: Math.min(330, size.width - 20),
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
                  <Text size={11} color={notice.error ? C.red : C.text}>
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
