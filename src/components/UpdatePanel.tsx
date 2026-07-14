import { RefreshCw } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import type { CheckResult, UpdateState } from "@/lib/update-manager"

interface UpdatePanelProps {
  state: UpdateState
  enabled: boolean
  onCheck: () => Promise<CheckResult>
  onShowUpdate: () => void
}

export function UpdatePanel({ state, enabled, onCheck, onShowUpdate }: UpdatePanelProps) {
  const handleCheck = async () => {
    if (state.candidate) {
      onShowUpdate()
      return
    }
    const result = await onCheck()
    if (result === "current") {
      toast.success("已是最新版本")
    } else if (result === "error") {
      toast.error("检查更新失败", { description: "请检查网络连接后重试" })
    }
  }

  const statusText = !enabled
    ? "开发模式不检查在线更新"
    : state.phase === "checking"
      ? "正在检查更新..."
      : state.candidate
        ? `发现新版本 ${state.candidate.version}`
        : state.lastCheckedAt
          ? `最近检查：${state.lastCheckedAt.toLocaleString()}`
          : "尚未完成检查"

  return (
    <div className="rounded-lg border p-4 mb-4">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold">软件更新</h2>
          <p className="text-sm text-muted-foreground mt-1">
            当前版本 {state.currentVersion || "读取中..."} · {statusText}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={handleCheck}
          disabled={!enabled || state.phase === "checking" || state.phase === "downloading" || state.phase === "installing"}
        >
          <RefreshCw className={`h-4 w-4 ${state.phase === "checking" ? "animate-spin" : ""}`} />
          {state.candidate ? "查看更新" : "检查更新"}
        </Button>
      </div>
      {state.phase === "error" && !state.candidate && state.error && (
        <p className="text-sm text-destructive mt-2">{state.error}</p>
      )}
    </div>
  )
}
