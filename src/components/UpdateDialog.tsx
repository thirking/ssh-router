import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import type { UpdateState } from "@/lib/update-manager"

interface UpdateDialogProps {
  state: UpdateState
  onInstall: () => Promise<void>
  onDismiss: () => Promise<void>
}

function formatBytes(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function UpdateDialog({ state, onInstall, onDismiss }: UpdateDialogProps) {
  const candidate = state.candidate
  const busy = state.phase === "downloading" || state.phase === "installing"
  const progress = state.totalBytes
    ? Math.min(100, Math.round((state.downloadedBytes / state.totalBytes) * 100))
    : null

  return (
    <Dialog
      open={candidate !== null && state.dialogOpen}
      onOpenChange={open => {
        if (!open && !busy) void onDismiss()
      }}
    >
      <DialogContent
        showCloseButton={!busy}
        onEscapeKeyDown={event => {
          if (busy) event.preventDefault()
        }}
        onPointerDownOutside={event => {
          if (busy) event.preventDefault()
        }}
      >
        <DialogHeader>
          <DialogTitle>
            {busy ? "正在更新 SSH Router" : `发现新版本 ${candidate?.version ?? ""}`}
          </DialogTitle>
          <DialogDescription>
            当前版本 {candidate?.currentVersion ?? state.currentVersion}
            {candidate?.date ? ` · 发布于 ${new Date(candidate.date).toLocaleString()}` : ""}
          </DialogDescription>
        </DialogHeader>

        {state.phase === "downloading" || state.phase === "installing" ? (
          <div className="space-y-2">
            <div className="h-2 overflow-hidden rounded-full bg-muted">
              <div
                className={`h-full bg-primary transition-all ${progress === null ? "w-1/3 animate-pulse" : ""}`}
                style={progress === null ? undefined : { width: `${progress}%` }}
              />
            </div>
            <p className="text-sm text-muted-foreground">
              {state.phase === "installing"
                ? "下载完成，正在安装并准备重启..."
                : progress !== null
                  ? `${progress}%（${formatBytes(state.downloadedBytes)} / ${formatBytes(state.totalBytes ?? 0)}）`
                  : `已下载 ${formatBytes(state.downloadedBytes)}`}
            </p>
          </div>
        ) : (
          <div className="max-h-64 overflow-y-auto rounded-md bg-muted p-3">
            <p className="whitespace-pre-wrap text-sm">{candidate?.body?.trim() || "此版本未提供更新说明。"}</p>
          </div>
        )}

        {state.phase === "error" && state.error && (
          <p className="text-sm text-destructive">更新失败：{state.error}</p>
        )}

        {!busy && (
          <DialogFooter>
            <Button variant="outline" onClick={() => void onDismiss()}>稍后</Button>
            <Button onClick={() => void onInstall()}>
              {state.phase === "error" ? "重试更新" : "立即更新"}
            </Button>
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  )
}
